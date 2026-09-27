//! Operation selection: generate only the operations a client uses, the types they
//! reach, and each type only in the direction it travels. A client of a handful of
//! Konnektor operations then compiles a few thousand lines instead of the whole
//! specification.

use std::collections::{HashMap, HashSet};

use anyhow::{Result, bail, ensure};
use indexmap::IndexMap;
use serde::Deserialize;

use crate::ir::{FieldType, ItemKind, Payload, Program};

/// A selection file (`--select`).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    /// Emit the async port traits; clients that call through a generic
    /// `SoapRequest` transport do not need them.
    #[serde(default = "yes")]
    pub ports: bool,
    pub services: Vec<SelectedService>,
}

fn yes() -> bool {
    true
}

/// The operations of one service version, as the Konnektor's service directory names
/// it.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectedService {
    /// Rust module path of the port, dot-separated, e.g. `gematik.conn.eventservice72`.
    pub module: String,
    /// Service name in the service directory, e.g. `EventService`.
    pub service: String,
    /// The `major.minor` version this module speaks, e.g. `7.2`.
    pub version: String,
    /// Operation names and how long each may take.
    pub operations: IndexMap<String, Timeout>,
}

/// How long an operation may take: `short` for lookups, `long` for anything that
/// waits for the card terminal or does card cryptography.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Timeout {
    Short,
    Long,
}

/// Where the operation sits in the service directory, and how long it may take.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationMeta {
    pub service: String,
    pub version: String,
    pub timeout: Timeout,
}

/// The directions a type travels in: written in requests, read from responses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Usage {
    pub read: bool,
    pub write: bool,
}

impl Usage {
    pub const BOTH: Usage = Usage {
        read: true,
        write: true,
    };
}

/// Keeps the selected operations (with their metadata) and the types they reach,
/// and records each type's direction.
pub fn apply(program: &mut Program, selection: &Selection) -> Result<()> {
    let mut wanted: HashMap<(Vec<String>, &str), (&SelectedService, Timeout)> = HashMap::new();
    for service in &selection.services {
        let module: Vec<String> = service.module.split('.').map(str::to_owned).collect();
        ensure!(
            service.version.split('.').count() == 2,
            "{}: version {:?} is not major.minor",
            service.module,
            service.version
        );
        for (name, timeout) in &service.operations {
            let previous = wanted.insert((module.clone(), name.as_str()), (service, *timeout));
            ensure!(
                previous.is_none(),
                "{}: {name} is selected twice",
                service.module
            );
        }
    }

    let mut found = HashSet::new();
    for port in &mut program.ports {
        port.operations.retain_mut(|op| {
            let Some((service, timeout)) = wanted.get(&(port.module.clone(), op.name.as_str()))
            else {
                return false;
            };
            found.insert((port.module.clone(), op.name.clone()));
            op.meta = Some(OperationMeta {
                service: service.service.clone(),
                version: service.version.clone(),
                timeout: *timeout,
            });
            true
        });
    }
    program.ports.retain(|port| !port.operations.is_empty());
    let mut missing: Vec<String> = wanted
        .keys()
        .filter(|(module, name)| !found.contains(&(module.clone(), (*name).to_owned())))
        .map(|(module, name)| format!("{}.{name}", module.join(".")))
        .collect();
    if !missing.is_empty() {
        missing.sort();
        bail!(
            "selected operations not in the specification: {}",
            missing.join(", ")
        );
    }

    let operations = program.ports.iter().flat_map(|p| &p.operations);
    let requests: Vec<&Payload> = operations.clone().map(|op| &op.input).collect();
    let responses: Vec<&Payload> = operations
        .flat_map(|op| std::iter::once(&op.output).chain(&op.faults))
        .collect();
    let write = reachable(program, &requests);
    let read = reachable(program, &responses);
    program
        .items
        .retain(|key, _| read.contains(key) || write.contains(key));
    program.usage = Some(
        program
            .items
            .keys()
            .map(|key| {
                let usage = Usage {
                    read: read.contains(key),
                    write: write.contains(key),
                };
                (key.clone(), usage)
            })
            .collect(),
    );
    program.ports_as_traits = selection.ports;
    Ok(())
}

/// Every schema key reachable from `roots` through fields and aliases.
fn reachable(program: &Program, roots: &[&Payload]) -> HashSet<String> {
    let mut seen = HashSet::new();
    let mut stack: Vec<&str> = roots.iter().map(|p| p.key.as_str()).collect();
    while let Some(key) = stack.pop() {
        if !seen.insert(key.to_owned()) {
            continue;
        }
        match program.items.get(key).map(|item| &item.kind) {
            Some(ItemKind::Struct(fields)) => {
                stack.extend(fields.iter().filter_map(|f| match &f.ty {
                    FieldType::Named(target) => Some(target.as_str()),
                    _ => None,
                }))
            }
            Some(ItemKind::Alias(target)) => stack.push(target),
            Some(ItemKind::Enum(_)) | None => {}
        }
    }
    seen
}
