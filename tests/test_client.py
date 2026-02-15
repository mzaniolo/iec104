import c104
import random
import time
import asyncio
import concurrent.futures
import functools
import logging

def async_exception_handler(task: asyncio.Future):
    """
    Callback to log unhandled exceptions in scheduled async tasks.

    Required when starting asyncio coroutines from non-async (C++) callbacks,
    using `run_coroutine_threadsafe`.
    """
    try:
        task.result()
    except (asyncio.CancelledError, concurrent.futures.CancelledError):
        # Silently suppress cancellation
        return
    except Exception:
        logging.error(f"Unhandled exception in coroutine:", exc_info=True)

async def async_measurement(point: c104.Point, message: c104.IncomingMessage) -> None:
    """
    Coroutine to handle incoming measurement updates.

    Simulates processing delay or background I/O after receiving a value from the C104 server.
    """
    print(f"{point.type} MEASUREMENT received on IOA: {point.io_address}, details: {point.info}")

    # Optional debug:
    # print(f"Raw APDU: {message.raw.hex()}")
    # print("Explained:", c104.explain_bytes(apdu=message.raw))

    await asyncio.sleep(3)  # Simulate processing delay
    print("Measurement handling completed after 3 seconds")


def on_receive_point(point: c104.Point, previous_info: c104.Information,
                     message: c104.IncomingMessage, loop: asyncio.AbstractEventLoop) -> c104.ResponseState:
    """
    Synchronous C++-side callback triggered on point update.

    Uses `run_coroutine_threadsafe` to schedule the async measurement handler in the asyncio loop.
    """
    future = asyncio.run_coroutine_threadsafe(async_measurement(point, message), loop)
    future.add_done_callback(async_exception_handler)

    # Response must be returned immediately, so the coroutine does not block this call.
    return c104.ResponseState.SUCCESS


async def main():
    loop = asyncio.get_event_loop()

    client = c104.Client(tick_rate_ms=1000, command_timeout_ms=5000)
    connection = client.add_connection(ip="127.0.0.1", port=2404, init=c104.Init.NONE)
    station = connection.add_station(common_address=47)
    point_1 = station.add_point(io_address=1, type=c104.Type.M_SP_NA_1)
    point_1.on_receive(callable=functools.partial(on_receive_point, loop=loop))
    point_3 = station.add_point(io_address=3, type=c104.Type.M_SP_NA_1)
    point_3.on_receive(callable=functools.partial(on_receive_point, loop=loop))
    client.start()

    while connection.state != c104.ConnectionState.OPEN:
        print(f"Waiting for connection to {connection.ip}:{connection.port}...")
        await asyncio.sleep(1)

    time.sleep(1)

    # --- Keep Alive to Process Incoming Data ---
    for second in range(60):
        if connection.state != c104.ConnectionState.OPEN:
            print("Connection closed.")
            break
        print(f"Client active... ({second + 1}s)")
        await asyncio.sleep(1)

    print("Disconnected")

if __name__ == "__main__":
    c104.set_debug_mode(c104.Debug.Server|c104.Debug.Point|c104.Debug.Callback)

    logging.basicConfig(level=logging.INFO)
    asyncio.run(main())
