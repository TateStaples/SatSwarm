# SatSwarm
Project for architectural simulation of a grid of DPLL Sat solver modules

## Overview
SatSwarm is an architectural simulation of a distributed SAT solver grid. The project implements a hardware-accelerated approach to solving boolean satisfiability (SAT) problems using a network of DPLL (Davis–Putnam–Logemann–Loveland) solver nodes.

### Key Features
- Grid-based architecture with configurable dimensions
- Work-stealing protocol for load balancing between nodes
- Cycle-accurate simulation in Rust
- Support for CNF (Conjunctive Normal Form) problem representation
- Distributed clause sharing and conflict resolution

## Getting Started

### Prerequisites
- Rust toolchain (2021 edition)
- Cargo package manager

### Installation
