# IEC104

A rust implementation of the [IEC-60870-5-104](https://en.wikipedia.org/wiki/IEC_60870-5#IEC_60870-5-104) protocol.

This create provides a client and (soon) a server that implements the IEC104 protocol. Some tests were made using the [c104](https://pypi.org/project/c104/) python library but some error may still arise. Despite is already working this is still a work in progress and the interfaces may change.

Contributions are welcome and encourage!
---------------
Run examples :
---------------
// to launch a server :
cargo run --axample server
// to launch a client :
cargo run --example client

