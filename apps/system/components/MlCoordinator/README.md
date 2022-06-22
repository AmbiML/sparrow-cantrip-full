# Crate descriptions

## Uses cantrip-os-common, not unit testable

cantrip-ml-interface: Outside interface used by external clients
cantrip-ml-component: Camkes interface, locks MLCoord
cantrip-ml-coordinator: Main point of logic

## Unit testable

cantrip-ml-shared: Shared structs used in most other crates
cantrip-ml-support: Unit testable code

## HAL

cantrip-vec-core: The HAL for the Vector Core
fake-vec-core: A stubbed out version of cantrip-vec-core
