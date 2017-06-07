# Nero Core

### What is Nero?
Nero is (going to be) an IRC Pseudo Server based on Tokio with a plugin-based API to write service bots with. Nero Core is just the core IO processesing and state tracking program. On its own, Nero Core won't add any bots. Bots will be done by adding/creating reloadable plugins, since the point of Core is to keep it as simple as possible.

### What **ISN'T*** Nero?
At this time, Nero is not planned to be a drop-in replacement for current services packages, but more or a way to create bots with network power.

<hr>

Future feature list includes:

* Rust API
* Embedded Python API

Things needed to be done:

* TS6 Protocol support
* InspIRCd Protocol support
* Plugin management
* Logging
* P10 Gline handling
* Better Protocol API

**Nero is currently still in its infancy with a lot of core code still yet to be written**
