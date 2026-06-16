# cogito

Operational TBox CLI for the wintermute box.

Ships a declarative [TOML spec](spec/cogito.toml) of the box's own operational world —
classes like `Daemon`, `Socket`, `Bus`, `Tool`, `Repo`, `Session`, `KernelPrimitive`,
`Healthcheck`, and `BusHealthcheck` — all BFO-grounded and forged to OWL 2 DL RDF/XML.

IRI base: `https://wintermute.local/cogito#`

## Usage

```
cogito tbox build [--out cogito.owl]   # forge the TBox to OWL 2 DL XML
cogito tbox check [--spec <dir>]       # validate without emitting
cogito tbox stats <file>               # class/property/axiom counts
```

## Why

The recalld healthcheck bug (a bus healthcheck on a daemon that writes to a socket
and never registers on the bus) is a *category error* — exactly the kind a TBox with
a `BusHealthcheck ⊑ ∃healthcheckedBy⁻.BusRegistrant` axiom catches at reasoning time.

## TBox highlights

- `Daemon ⊑ BFO:process`, `Socket ⊑ BFO:continuant`, `Session ⊑ BFO:process`
- `dependsOn` is `owl:TransitiveProperty`
- `BusRegistrant ≡ Daemon ⊓ ∃registersOn.Bus`
- `BusHealthcheck ⊑ ∃healthcheckedBy⁻.BusRegistrant`

Validation: `ousia-reason check --owl cogito.owl` (OWL 2 DL CONFORMANT).

## Build

```
cargo build --release
cp target/release/cogito ~/.local/bin/
```

MSRV: 1.85
