"""
SCADA client for manual / CI checks against `cargo run --example rtu_server`.

Expected server: `iec104` RTU example on TCP (default 127.0.0.1:2404), common address **47**:
  - IOA **1**, **3**: `M_SP_NA_1`
  - IOA **11**: `M_ME_NC_1`

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


async def main():
    loop = asyncio.get_running_loop()

    client = c104.Client(tick_rate_ms=1000, command_timeout_ms=5000)
    # General interrogation: exercises RTU activation confirmation + interrogation ASDUs.
    connection = client.add_connection(ip=HOST, port=PORT, init=c104.Init.INTERROGATION)
    station = connection.add_station(common_address=47)

    for ioa in (1, 3):
        p = station.add_point(io_address=ioa, type=c104.Type.M_SP_NA_1)
        p.on_receive(callable=make_on_receive(loop))

    p11 = station.add_point(io_address=11, type=c104.Type.M_ME_NC_1)
    p11.on_receive(callable=make_on_receive(loop))

    client.start()

    deadline = time.monotonic() + 30.0
    while connection.state != c104.ConnectionState.OPEN:
        if time.monotonic() > deadline:
            print(f"Timeout waiting for OPEN to {HOST}:{PORT} — is `rtu_server` running?", file=sys.stderr)
            sys.exit(2)
        print(f"Waiting for connection to {HOST}:{PORT} (state={connection.state!r})...")
        await asyncio.sleep(0.5)

    print(f"Connected to {HOST}:{PORT}, CA=47. Listening ~{TEST_SECONDS}s for measurements (GI + spontaneous)...")
    await asyncio.sleep(1)

    for second in range(TEST_SECONDS):
        if connection.state != c104.ConnectionState.OPEN:
            print("Connection closed.")
            break
        print(f"  ... {second + 1}/{TEST_SECONDS}s (measurements received: {_measurement_count})")
        await asyncio.sleep(1)

    print(f"Done. Total measurement callbacks: {_measurement_count}")
    if REQUIRE_UPDATES and _measurement_count == 0:
        print("RTU_TEST_REQUIRE_UPDATES=1 but no measurements — failing.", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    c104.set_debug_mode(c104.Debug.Server | c104.Debug.Point | c104.Debug.Callback)
    logging.basicConfig(level=logging.INFO)
    asyncio.run(main())
