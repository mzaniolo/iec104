"""
SCADA client for manual / CI checks against `cargo run --example rtu_server`.

Expected server: `iec104` RTU example on TCP (default 127.0.0.1:2404), common address **47**
(see `examples/rtu_server.rs`):

  - IOA **1**, **3**: `M_SP_NA_1` — `C_SC_NA_1`
  - IOA **11**: `M_ME_NC_1` — `C_SE_NC_1`
  - IOA **20**, **21**: `M_ME_NA_1` / `M_ME_NB_1` — `C_SE_NA_1` / `C_SE_NB_1`
  - IOA **30**: `M_IT_NA_1` (counter interrogation group 1)
  - IOA **40**: `M_ME_NA_1` (general interrogation group 2)

**Two TCP clients**: c104 allows only one point per IOA per station. Monitoring uses `M_*`; commands
use `C_*` at the same IOAs as `MapCommandsToSameIoaMonitoring`.

After connect, this script also checks **clock synchronization**, **counter interrogation**,
**group-2 general interrogation**, and optionally **test command** (`Connection.test`) when the
installed `c104` version exposes them.

Requires [iec104-python](https://iec104-python.readthedocs.io/latest/) (`c104` in the repo **`.venv`**, or on `PYTHONPATH`).

`tests/run_rtu_e2e.sh` uses `$REPO/.venv/bin/python` when that venv can `import c104`. Override with **`RTU_TEST_PYTHON`**.

Environment:
  IEC104_HOST       — bind / connect host (default 127.0.0.1)
  IEC104_PORT       — port (default 2404)
  RTU_TEST_SECONDS  — how long to stay connected after checks (default 12)
  RTU_TEST_REQUIRE_UPDATES — if set to "1", exit 1 if no measurement was received
  RTU_TEST_PYTHON   — path to Python interpreter (optional; defaults to `.venv` then `python3`)
"""

import asyncio
import concurrent.futures
import logging
import os
import sys
import time
from collections import defaultdict

import c104

HOST = os.environ.get("IEC104_HOST", "127.0.0.1")
PORT = int(os.environ.get("IEC104_PORT", "2404"))
TEST_SECONDS = int(os.environ.get("RTU_TEST_SECONDS", "12"))
REQUIRE_UPDATES = os.environ.get("RTU_TEST_REQUIRE_UPDATES", "") == "1"

_measurement_count = 0
_command_transmits_failed = 0
# (io_address, Type) -> number of on_receive callbacks
_rx_type_ioa = defaultdict(int)


def async_exception_handler(task):
    """Log unhandled exceptions from run_coroutine_threadsafe."""
    try:
        task.result()
    except (asyncio.CancelledError, concurrent.futures.CancelledError):
        return
    except Exception:
        logging.error("Unhandled exception in coroutine:", exc_info=True)


async def async_measurement(point, message):
    """Async follow-up after a measurement callback (non-blocking for the C++ callback)."""
    global _measurement_count
    _measurement_count += 1
    _rx_type_ioa[(point.io_address, point.type)] += 1
    print(
        f"[{_measurement_count}] {point.type} IOA={point.io_address} "
        f"info={point.info!r} value={getattr(point, 'value', None)!r}"
    )
    await asyncio.sleep(0.05)


def make_on_receive(loop):
    """
    Return a **new** callback per point: c104 replaces any prior registration using the same
    callable object. Signatures must match c104's expected annotations (no PEP 563 string types).
    """

    def on_receive_point(
        point: c104.Point,
        previous_info: c104.Information,
        message: c104.IncomingMessage,
    ) -> c104.ResponseState:
        future = asyncio.run_coroutine_threadsafe(async_measurement(point, message), loop)
        future.add_done_callback(async_exception_handler)
        return c104.ResponseState.SUCCESS

    return on_receive_point


def transmit_command(point: c104.Point, label: str) -> None:
    """Send one process command; counts failures for the summary line."""
    global _command_transmits_failed
    ok = point.transmit(cause=c104.Cot.ACTIVATION)
    print(f"  command transmit {label!r}: {'ok' if ok else 'FAILED'}")
    if not ok:
        _command_transmits_failed += 1


def _connection_usable(state: c104.ConnectionState) -> bool:
    """Data exchange can run in OPEN or OPEN_MUTED; do not block only on OPEN."""
    return state in (c104.ConnectionState.OPEN, c104.ConnectionState.OPEN_MUTED)


async def wait_open(connection: c104.Connection, label: str, deadline_s: float = 30.0) -> None:
    deadline = time.monotonic() + deadline_s
    while not _connection_usable(connection.state):
        if time.monotonic() > deadline:
            print(
                f"Timeout waiting for {label} OPEN to {HOST}:{PORT} — is `rtu_server` running?",
                file=sys.stderr,
            )
            sys.exit(2)
        print(f"Waiting for {label} to {HOST}:{PORT} (state={connection.state!r})...")
        await asyncio.sleep(0.5)


def _run_rtu_server_feature_checks(monitor_conn: c104.Connection) -> bool:
    """
    Exercise RTU system handlers: clock sync, counter interrogation, group GI, test command.
    Returns False if any required step fails.
    """
    ok = True

    if not hasattr(monitor_conn, "clock_sync"):
        print("  skip: connection has no clock_sync (c104 too old?)")
    else:
        print("  clock_sync(CA=47)...")
        if not monitor_conn.clock_sync(common_address=47):
            print("  ERROR: clock_sync returned False", file=sys.stderr)
            ok = False

    if not hasattr(monitor_conn, "counter_interrogation"):
        print("  skip: connection has no counter_interrogation")
    else:
        print("  counter_interrogation(GENERAL, READ)...")
        t30_before = _rx_type_ioa[(30, c104.Type.M_IT_NA_1)]
        if not monitor_conn.counter_interrogation(
            common_address=47,
            cause=c104.Cot.ACTIVATION,
            qualifier=c104.Rqt.GENERAL,
            freeze=c104.Frz.READ,
        ):
            print("  ERROR: counter_interrogation returned False", file=sys.stderr)
            ok = False
        time.sleep(0.7)
        t30_after = _rx_type_ioa[(30, c104.Type.M_IT_NA_1)]
        if t30_after <= t30_before:
            print(
                "  ERROR: expected at least one M_IT_NA_1 update for IOA 30 after counter CI",
                file=sys.stderr,
            )
            ok = False
        else:
            print(f"    M_IT_NA_1 IOA=30 callbacks: {t30_before} -> {t30_after}")

    if not hasattr(monitor_conn, "interrogation"):
        print("  skip: connection has no interrogation")
    else:
        print("  interrogation(Qoi.GROUP_2)...")
        t40_before = _rx_type_ioa[(40, c104.Type.M_ME_NA_1)]
        if not monitor_conn.interrogation(
            common_address=47,
            cause=c104.Cot.ACTIVATION,
            qualifier=c104.Qoi.GROUP_2,
        ):
            print("  ERROR: group-2 interrogation returned False", file=sys.stderr)
            ok = False
        time.sleep(0.7)
        t40_after = _rx_type_ioa[(40, c104.Type.M_ME_NA_1)]
        if t40_after <= t40_before:
            print(
                "  ERROR: expected at least one M_ME_NA_1 update for IOA 40 after group-2 GI",
                file=sys.stderr,
            )
            ok = False
        else:
            print(f"    M_ME_NA_1 IOA=40 callbacks: {t40_before} -> {t40_after}")

    if hasattr(monitor_conn, "test"):
        # iec104-python registers the success waiter as C_TS_TA_1 even when
        # with_time=False (C_TS_NA_1), so ACT_CON never matches and test() times
        # out. Use the time-tagged variant so cmdId matches the response.
        print("  test command (C_TS_TA_1, with time tag)...")
        if not monitor_conn.test(common_address=47, with_time=True):
            print("  ERROR: test() returned False", file=sys.stderr)
            ok = False
    else:
        print("  skip: connection has no test()")

    return ok


async def main():
    loop = asyncio.get_running_loop()

    monitor_client = c104.Client(tick_rate_ms=1000, command_timeout_ms=5000)
    # General interrogation: exercises RTU activation confirmation + interrogation ASDUs.
    monitor_conn = monitor_client.add_connection(ip=HOST, port=PORT, init=c104.Init.INTERROGATION)
    monitor_station = monitor_conn.add_station(common_address=47)

    for ioa in (1, 3):
        p = monitor_station.add_point(io_address=ioa, type=c104.Type.M_SP_NA_1)
        p.on_receive(callable=make_on_receive(loop))

    p11 = monitor_station.add_point(io_address=11, type=c104.Type.M_ME_NC_1)
    p11.on_receive(callable=make_on_receive(loop))

    p20 = monitor_station.add_point(io_address=20, type=c104.Type.M_ME_NA_1)
    p20.on_receive(callable=make_on_receive(loop))
    p21 = monitor_station.add_point(io_address=21, type=c104.Type.M_ME_NB_1)
    p21.on_receive(callable=make_on_receive(loop))

    p30 = monitor_station.add_point(io_address=30, type=c104.Type.M_IT_NA_1)
    p30.on_receive(callable=make_on_receive(loop))
    p40 = monitor_station.add_point(io_address=40, type=c104.Type.M_ME_NA_1)
    p40.on_receive(callable=make_on_receive(loop))

    # Second connection: command-only points at the same IOAs (see module docstring).
    cmd_client = c104.Client(tick_rate_ms=1000, command_timeout_ms=5000)
    cmd_conn = cmd_client.add_connection(ip=HOST, port=PORT, init=c104.Init.NONE)
    cmd_station = cmd_conn.add_station(common_address=47)

    cmd_sp_1 = cmd_station.add_point(io_address=1, type=c104.Type.C_SC_NA_1)
    cmd_sp_3 = cmd_station.add_point(io_address=3, type=c104.Type.C_SC_NA_1)
    cmd_me_11 = cmd_station.add_point(io_address=11, type=c104.Type.C_SE_NC_1)
    cmd_me_20 = cmd_station.add_point(io_address=20, type=c104.Type.C_SE_NA_1)
    cmd_me_21 = cmd_station.add_point(io_address=21, type=c104.Type.C_SE_NB_1)

    monitor_client.start()
    cmd_client.start()

    feature_checks_ok = True
    try:
        await wait_open(monitor_conn, "monitor connection")
        await wait_open(cmd_conn, "command connection")

        print(f"Connected to {HOST}:{PORT}, CA=47. Initial GI + measurements...")
        await asyncio.sleep(1.5)

        print("RTU system / interrogation feature checks...")
        loop = asyncio.get_running_loop()
        feature_checks_ok = await loop.run_in_executor(None, _run_rtu_server_feature_checks, monitor_conn)
        if not feature_checks_ok:
            print("One or more RTU feature checks failed.", file=sys.stderr)

        print("Sending process commands (second TCP client)...")
        cmd_sp_1.info = c104.SingleCmd(on=True, qualifier=c104.Qoc.NONE)
        transmit_command(cmd_sp_1, "C_SC_NA_1 IOA=1 ON")
        await asyncio.sleep(0.4)
        cmd_sp_3.info = c104.SingleCmd(on=True, qualifier=c104.Qoc.NONE)
        transmit_command(cmd_sp_3, "C_SC_NA_1 IOA=3 ON")
        await asyncio.sleep(0.4)
        cmd_me_11.info = c104.ShortCmd(target=77.5, qualifier=c104.UInt7(0))
        transmit_command(cmd_me_11, "C_SE_NC_1 IOA=11 -> 77.5")
        await asyncio.sleep(0.4)
        cmd_me_20.info = c104.NormalizedCmd(target=c104.NormalizedFloat(0.5), qualifier=c104.UInt7(0))
        transmit_command(cmd_me_20, "C_SE_NA_1 IOA=20 normalized 0.5")
        await asyncio.sleep(0.4)
        cmd_me_21.info = c104.ScaledCmd(target=c104.Int16(2100), qualifier=c104.UInt7(0))
        transmit_command(cmd_me_21, "C_SE_NB_1 IOA=21 scaled 2100")
        await asyncio.sleep(0.5)

        print(f"Listening ~{TEST_SECONDS}s for further measurements (spontaneous / ticks)...")
        for second in range(TEST_SECONDS):
            if not _connection_usable(monitor_conn.state):
                print("Monitor connection closed.")
                break
            print(f"  ... {second + 1}/{TEST_SECONDS}s (measurements received: {_measurement_count})")
            await asyncio.sleep(1)

        print(
            f"Done. Total measurement callbacks: {_measurement_count}; "
            f"command transmit failures: {_command_transmits_failed}"
        )
    finally:
        monitor_client.stop()
        cmd_client.stop()

    if REQUIRE_UPDATES and _measurement_count == 0:
        print("RTU_TEST_REQUIRE_UPDATES=1 but no measurements — failing.", file=sys.stderr)
        sys.exit(1)
    if _command_transmits_failed:
        print("One or more command transmits failed — failing.", file=sys.stderr)
        sys.exit(1)
    if not feature_checks_ok:
        print("RTU feature checks failed — failing.", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    c104.set_debug_mode(c104.Debug.Server | c104.Debug.Point | c104.Debug.Callback)
    logging.basicConfig(level=logging.INFO)
    asyncio.run(main())
