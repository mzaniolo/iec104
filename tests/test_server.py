import c104
import random
import time


def on_step_command(point: c104.Point, previous_info: c104.Information, message: c104.IncomingMessage) -> c104.ResponseState:
    """ handle incoming regulating step command
    """
    print("{0} STEP COMMAND on IOA: {1}, message: {2}, previous: {3}, current: {4}".format(point.type, point.io_address, message, previous_info, point.info))

    if point.value == c104.Step.LOWER:
        # do something
        return c104.ResponseState.SUCCESS

    if point.value == c104.Step.HIGHER:
        # do something
        return c104.ResponseState.SUCCESS

    return c104.ResponseState.FAILURE

def on_dp_command(point: c104.Point, previous_info: c104.Information, message: c104.IncomingMessage) -> c104.ResponseState:
    """ handle incoming regulating step command
    """
    print("{0} DP COMMAND on IOA: {1}, message: {2}, previous: {3}, current: {4}".format(point.type, point.io_address, message, previous_info, point.info))

    return c104.ResponseState.SUCCESS


def before_auto_transmit(point: c104.Point) -> None:
    """ update point value before transmission
    """
    point.value = random.random() * 100
    print("{0} BEFORE AUTOMATIC REPORT on IOA: {1} VALUE: {2}".format(point.type, point.io_address, point.value))

def before_auto_transmit_dp(point: c104.Point) -> None:
    """ update point value before transmission
    """
    point.value = random.choice([c104.Double.ON, c104.Double.OFF,  c104.Double.INTERMEDIATE])
    print("{0} BEFORE AUTOMATIC REPORT on IOA: {1} VALUE: {2}".format(point.type, point.io_address, point.value))

def before_read(point: c104.Point) -> None:
    """ update point value before transmission
    """
    point.value = random.random() * 100
    print("{0} BEFORE READ or INTERROGATION on IOA: {1} VALUE: {2}".format(point.type, point.io_address, point.value))


def main():
    # server and station preparation
    server = c104.Server()
    station = server.add_station(common_address=47)


    # monitoring point preparation
    point = station.add_point(io_address=11, type=c104.Type.M_ME_NC_1, report_ms=700)
    point.on_before_auto_transmit(callable=before_auto_transmit)
    point.on_before_read(callable=before_read)

    point = station.add_point(io_address=12, type=c104.Type.M_DP_NA_1, report_ms=1000)
    point.on_before_auto_transmit(callable=before_auto_transmit_dp)
    point.on_before_read(callable=before_read)

    # command point preparation
    command = station.add_point(io_address=13, type=c104.Type.C_RC_TA_1)
    command.on_receive(callable=on_step_command)

    command = station.add_point(io_address=14, type=c104.Type.C_DC_NA_1)
    command.on_receive(callable=on_dp_command)

    # start
    server.start()

    while not server.has_active_connections:
        print("Waiting for connection")
        time.sleep(1)

    time.sleep(1)


    while server.has_open_connections :
        print("Keep alive until disconnected")
        time.sleep(1)

    print("Disconnected")

if __name__ == "__main__":
    c104.set_debug_mode(c104.Debug.Server|c104.Debug.Point|c104.Debug.Callback)
    main()
