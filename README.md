# IEC104

A rust implementation of the [IEC-60870-5-104](https://en.wikipedia.org/wiki/IEC_60870-5#IEC_60870-5-104) protocol.

This create provides a client and (soon) a server that implements the IEC104 protocol. Some tests were made using the [c104](https://pypi.org/project/c104/) python library but some error may still arise. Despite is already working this is still a work in progress and the interfaces may change.

Contributions are welcome and encourage!

## Pre-commit usage

A set of [pre-commits](https://pre-commit.com) hooks are provided

1. If not installed, install with your package manager, or `pip install --user pre-commit`
2. Run `pre-commit autoupdate` to update the pre-commit config to use the newest template
3. Run `pre-commit install` to install the pre-commit hooks to your local environment
