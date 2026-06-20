# cogito

Forges the wintermute box's operational world — its daemons, sockets, buses, tools, sessions — into an OWL 2 DL ontology, so the relationships between them are checkable instead of assumed.

## Why it exists

Some bugs are category errors. The one that prompted this: a *bus* healthcheck pointed at a daemon that writes to a socket and never registers on the bus. Nothing crashed; the check just asked the wrong question of the wrong kind of thing. That class of mistake is invisible to a test suite but obvious to a reasoner, because it's a contradiction in *types*, not in values.

`cogito` writes the box's operational types down as a TBox. Once `BusHealthcheck` is defined as something that must be healthchecked by a `BusRegistrant`, a healthcheck attached to a non-registrant is no longer a judgment call — it's an inconsistency a DL reasoner will name.

## What it is

A single declarative [TOML spec](spec/cogito.toml) of the operational world, plus a CLI that forges it to OWL 2 DL RDF/XML via `horned-owl`. The spec is BFO-grounded (it imports BFO 2020). Eleven classes — `Daemon`, `Unit`, `Socket`, `Bus`, `Tool`, `Repo`, `Session`, `KernelPrimitive`, `Healthcheck`, `BusHealthcheck`, `BusRegistrant` — and seven object properties.

IRI base: `https://wintermute.local/cogito#`

## Install

Requires `cargo` / `rustc` 1.85+.

```sh
git clone https://github.com/j0yen/cogito.git
cd cogito
cargo build --release
cp target/release/cogito ~/.local/bin/
```

## Usage

```
cogito tbox build [--out cogito.owl]   # forge the TBox to OWL 2 DL RDF/XML
cogito tbox check  [--spec <dir>]      # parse + validate the spec, emit nothing
cogito tbox stats  <file>              # class / property / axiom counts of a built ontology
```

`build` on the built-in spec produces 11 classes, 7 object properties, and 56 axioms. If `ousia-reason` is on `$PATH`, `build` runs an OWL 2 DL profile check on the output; if it isn't, the check is skipped and the build still emits. `check` validates only that the spec loads — the DL reasoning lives in the reasoner, not here.

```sh
$ cogito tbox stats cogito.owl
classes:            11
object_properties:  7
total_axioms:       56
```

## The axioms that matter

A few entailments do the real work:

- `Daemon ⊑ BFO:process`, `Socket ⊑ BFO:continuant`, `Session ⊑ BFO:process` — operational things placed under the right BFO category.
- `dependsOn` is an `owl:TransitiveProperty` — dependency chains close on their own.
- `BusRegistrant ≡ Daemon ⊓ ∃registersOn.Bus` — what it means to be on the bus, defined rather than asserted.
- `BusHealthcheck ⊑ ∃healthcheckedBy⁻.BusRegistrant` — the axiom that catches the healthcheck category error.

## Where it fits

`cogito` builds the ontology; `ousia-reason check --owl cogito.owl` reasons over it (reports OWL 2 DL conformance). Sibling tooling in the `ousia` line owns forging-by-spec and reasoning; `cogito` is the operational-world spec for the wintermute box specifically.

## Status

`v0.1.0`. The spec, the three subcommands, and the horned-owl emit path all work.

## License

MIT.
