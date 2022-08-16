# CantripOS System Component Model

This document lays out how we expect most components to be constructed in the
CantripOS / CAmkES world. Note that we say "most" here, because not everything can
be so dogmatic.

In general, most of our components are divided into three crates: interface,
component, and manager. These crates each provide slightly different interfaces
into the C bindings that are constructed by the build system.

Typically, the source tree is organized like this:

```
system/
├ components/
│  ├ SecurityCoordinator/
│  │  ├ Cargo.toml
│  │  ├ SecurityCoordinator.camkes
│  │  ├ cantrip-security-component/
│  │  ├ cantrip-security-coordinator/
│  │  └ cantrip-security-interface/
│  ┆
│
├ interfaces/
│  ├ SecurityCoordinatorInterface.camkes
│  ┆
│
└ system.camkes
```

`system.camkes` is the toplevel CAmkES Assembly, that defines which components
exist in the entire system, and which components speak to which other
components.

The CAmkES interfaces for each component are defined in the `interfaces/`
directory, named after its specific component
(`SecurityCoordinatorInterface.camkes` for this particular instance). These
simply define the interface of verbs other components can call out to, and helps
to define the client-side C interface for the component.

The crates that make up the code for each component, however, exist in the
`components/` directory, named after their component. Typically this is a
crate-of-crates configuration, and includes another camkes file defining the
implementation of the component, and thus defining the server-side C interface.

## Interface

The interface crate defines the native Rust client APIs for a given component.
This helps to isolate clients from the underlying C and IPC mechanics,
effectively creating a mirror of the CAmkES interface in Rust. This is due to
the fact that CAmkES does not (at the moment -- we'll address that later) create
Rust source code from its assembly descriptions.

In CAmkES, you explicitly define which components a component will speak to, and
CAmkES generates stub C bindings for you that then cross the seL4 IPC boundary
(as defined in the CAmkES interface description in the interface directory).
Note that the RPC connections are not visible to the client or server, and the
interface method name defines the C function that an implementation must use.

The top-level functions defined here simply wrap the CAmkES stubs to call across
to the component, effectively creating a client-side interface, and wrapping up
the unsafe entry points.

Note that this wrapping of unsafe entry points into safe Rust calls does not
mean we've swept the tree for unsafe issues! We hope to investigate this in the
future.

## Component

The component crate defines the C to Rust interconnect for the server-side of
the component, or rather, it's implementation.

Inside this crate is a set of top-level functions, named from their CAmkES
IDL specifications.

## Manager

This crate contains the actual full Rust implementation of the component, and
typically includes the impl for the trait defined in the Component crate, though
this varies significantly.
