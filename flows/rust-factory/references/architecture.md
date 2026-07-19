# Architecture standards

The stable constraints every design and implementation must satisfy. Internalise these; do
not restate them in stage outputs.

## Hexagonal architecture, ECB roles

Follow Entity-Control-Boundary inside a hexagonal (ports-and-adapters) shape:

- **Entities** hold the business logic. Pure domain types and rules. Framework-free.
- **Controls** implement use cases, reaching the outside world only through **ports**
  (traits/interfaces the domain owns).
- **Boundaries** are the adapters: API, CLI, DB, clock, network. They implement the ports.

**All dependencies point inward.** Adapters depend on the domain; the domain depends on
nothing outward. The domain never imports a framework, a driver, or an I/O type.

## Functional core, imperative shell

The core is **immutable and effect-free**: given the same inputs it returns the same
outputs, touches no clock, no disk, no network, no global state. Push all mutation and I/O
to the edges (the boundaries). If an effect leaks into the core, the design is wrong — move
it out, do not comment around it.

## Organise by capability, not by layer

The project structure should scream what the system does, not which framework it uses. Group
code by business capability. One responsibility per file: if a file has two reasons to
change, split it.

## What a design output must show

- every spec requirement mapped to an entity, control, or boundary;
- the ports (the traits the domain owns) and which adapters implement them;
- the module/crate layout, with the dependency direction explicit and inward;
- for a brownfield change: how it fits the existing architecture without violating the
  inward rule — and, if the existing code already violates it, that is called out, not copied.
