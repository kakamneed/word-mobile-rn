# Migration Baseline Test Suite

This directory contains the frozen baseline samples that serve as the migration acceptance
criteria for the Flutter + Rust + Supabase rearchitecture.

## Purpose

Before any Flutter, Supabase, or bridge code is written, these baselines capture the current
learning truth from the Rust core. All future implementations must pass these baselines to
prove they preserve existing learning logic.

## Structure

```
fixtures/         Shared fixtures (seed version, clock, templates)
bootstrap/        Bootstrap state samples
today/            Today home state samples
study/            Study session lifecycle samples
recovery/         Session recovery and cancellation samples
wrong-words/      Wrong word state samples
reports/          Report aggregation samples
ai/               AI non-blocking behavior samples
runners/          Runner and comparison specifications
```

## Running

```bash
cargo test --test baseline_runner
```

## Contract

- Each sample directory contains: `input.json`, `expected-output.json`, `notes.md`
- The runner executes API calls against a seeded in-memory SQLite database
- Results are compared structurally, not by exact string match
- Unstable fields (timestamps, UUIDs) are normalized before comparison
