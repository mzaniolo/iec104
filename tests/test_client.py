"""
SCADA client for manual / CI checks against `cargo run --example rtu_server`.

Expected server: `iec104` RTU example on TCP (default 127.0.0.1:2404), common address **47**:
  - IOA **1**, **3**: `M_SP_NA_1`
  - IOA **11**: `M_ME_NC_1`
  - IOA **20**, **21**: `M_ME_NA_1` / `M_ME_NB_1` (set-point targets for `C_SE_NA_1` / `C_SE_NB_1`)

**Two TCP clients**: c104 allows only one point per IOA per station, and incoming ASDUs must match that
point's type. Monitoring uses `M_*` types; commands use `C_*` at the same IOAs as the RTU's
`MapCommandsToSameIoaMonitoring` preset (see `examples/rtu_server.rs`). A second `c104.Client` with
`Init.NONE` sends those commands on a parallel connection.

Requires [iec104-python](https://iec104-python.readthedocs.io/latest/) (`c104` in the repo **`.venv`**, or on `PYTHONPATH`).

`tests/run_rtu_e2e.sh` uses `$REPO/.venv/bin/python` when that venv can `import c104`. Override with **`RTU_TEST_PYTHON`**.

Environment:
  IEC104_HOST       — bind / connect host (default 127.0.0.1)
  IEC104_PORT       — port (default 2404)
  RTU_TEST_SECONDS  — how long to stay connected (default 12)
  RTU_TEST_REQUIRE_UPDATES — if set to "1", exit 1 if no measurement was received
  RTU_TEST_PYTHON   — path to Python interpreter (optional; defaults to `.venv` then `python3`)
"""

import asyncio
import concurrent.futures
import logging
import os
import sys
import time

import c104

HOST = os.environ.get("IEC104_HOST", "127.0.0.1")
PORT = int(os.environ.get("IEC104_PORT", "2404"))
TEST_SECONDS = int(os.environ.get("RTU_TEST_SECONDS", "12"))
REQUIRE_UPDATES = os.environ.get("RTU_TEST_REQUIRE_UPDATES", "") == "1"

# Shared across callbacks (single-threaded event loop)
_measurement_count = 0
_command_transmits_failed = 0


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

    try:
        await wait_open(monitor_conn, "monitor connection")
        await wait_open(cmd_conn, "command connection")

        print(f"Connected to {HOST}:{PORT}, CA=47. Listening ~{TEST_SECONDS}s for measurements (GI + spontaneous)...")
        await asyncio.sleep(1)

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


if __name__ == "__main__":
    c104.set_debug_mode(c104.Debug.Server | c104.Debug.Point | c104.Debug.Callback)
    logging.basicConfig(level=logging.INFO)
    asyncio.run(main())
