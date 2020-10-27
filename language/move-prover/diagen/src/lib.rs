// Copyright (c) The Libra Core Contributors
// SPDX-License-Identifier: Apache-2.0

use log::{error, info};
use regex::Regex;
use serde::{self, Deserialize, Serialize};
use std::{
    collections::{btree_map::Entry, BTreeMap, BTreeSet, VecDeque},
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

/// Options passed into the diagram generator.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DiagenOptions {
    pub generate_dot: bool,
    pub generate_pdf: bool,
    pub generate_svg: bool,
    pub generate_global_graph: bool,
}

impl Default for DiagenOptions {
    fn default() -> Self {
        Self {
            generate_dot: false,
            generate_pdf: false,
            generate_svg: false,
            generate_global_graph: false,
        }
    }
}

pub fn run_diagram_generator(
    inp_files: &[PathBuf],
    out_dir: &PathBuf,
    options: &DiagenOptions,
) -> anyhow::Result<()> {
    if (options.generate_pdf || options.generate_svg) && !is_command_available("dot") {
        error!("Command not found: dot\nTo install, `brew install graphviz`");
        return Err(anyhow::anyhow!("Command not found: dot"));
    }

    let mut dep_graph: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut dep_graph_inverse: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for path in inp_files.iter() {
        info!("Processed {:?}", path);
        let content = fs::read_to_string(path)?;

        let rex1 = Regex::new(r"(?m)^module\s+(\w+)\s*\{").unwrap();
        let caps = rex1.captures(&content);
        if caps.is_none() {
            // skip due to no module declaration found.
            continue;
        }
        let module_name = caps.unwrap()[1].to_string();
        if let Entry::Vacant(e) = dep_graph_inverse.entry(module_name.clone()) {
            e.insert(vec![]);
        }

        let mut dep_list: Vec<String> = Vec::new();
        // TODO: It is not covered for the cases where modules are used with full qualification
        //  without `use`. To address this issue, see `move-prover/src/lib.rs`.
        let rex2 = Regex::new(r"(?m)use 0x1::(\w+)\s*(;|:)").unwrap();
        for cap in rex2.captures_iter(&content) {
            let dep = cap.get(1).unwrap().as_str().to_string();
            dep_list.push(dep.clone());

            match dep_graph_inverse.entry(dep) {
                Entry::Vacant(e) => {
                    e.insert(vec![module_name.clone()]);
                }
                Entry::Occupied(mut e) => {
                    e.get_mut().push(module_name.clone());
                }
            }
        }
        dep_graph.insert(module_name, dep_list);
    }
    fs::create_dir_all(out_dir)?;

    if options.generate_global_graph {
        // Generate a diagram for the entire dependency graph.
        let mut dot_src: String = String::new();

        for (module, dep_list) in dep_graph.iter() {
            dot_src.push_str(&format!("    {}\n", module));
            for dep in dep_list.iter() {
                dot_src.push_str(&format!("    {} -> {}\n", module, dep));
            }
        }

        write_output(
            &out_dir.join("(EntireGraph)"),
            &format!("digraph G {{\n{}}}\n", dot_src),
            options,
        )?;
    }

    // Generate a .forward.dot file for the forward dependency graph per module.
    generate_diagram_per_module(&dep_graph, out_dir, options, true)?;

    // Generate a .backward.dot file for the backward dependency graph per module.
    generate_diagram_per_module(&dep_graph_inverse, out_dir, options, false)?;

    Ok(())
}

fn is_command_available(cmd: &str) -> bool {
    Command::new("type")
        .arg(cmd)
        .output()
        .expect("process failed to execute.")
        .status
        .success()
}

fn generate_diagram_per_module(
    graph: &BTreeMap<String, Vec<String>>,
    out_dir: &Path,
    options: &DiagenOptions,
    is_forward: bool,
) -> anyhow::Result<()> {
    let empty_list = Vec::new();

    for (module, _) in graph.iter() {
        // (breadth-first) search and gather the modules in `graph` which are reachable from `module`.
        let mut dot_src: String = String::new();
        let mut visited: BTreeSet<String> = BTreeSet::new();
        let mut queue: VecDeque<String> = VecDeque::new();

        visited.insert(module.clone());
        queue.push_back(module.clone());

        while let Some(d) = queue.pop_front() {
            let dep_list = graph.get(&d).unwrap_or(&empty_list);
            dot_src.push_str(&format!("    {}\n", d));
            for dep in dep_list.iter() {
                if is_forward {
                    dot_src.push_str(&format!("    {} -> {}\n", d, dep));
                } else {
                    dot_src.push_str(&format!("    {} -> {}\n", dep, d));
                }
                if !visited.contains(dep) {
                    visited.insert(dep.clone());
                    queue.push_back(dep.clone());
                }
            }
        }
        let out_file = out_dir.join(format!(
            "{}_{}",
            module,
            (if is_forward { "forward" } else { "backward" }).to_string()
        ));

        write_output(
            &out_file,
            &format!("digraph G {{\n{}}}\n", dot_src),
            options,
        )?;
    }
    Ok(())
}

fn write_output(out_path: &Path, dot_src: &str, options: &DiagenOptions) -> anyhow::Result<()> {
    if options.generate_dot {
        fs::write(out_path.with_extension("dot"), dot_src)?;
        info!("Generated {:?}", out_path.with_extension("dot"));
    }

    for (flag, ext) in &[(options.generate_pdf, "pdf"), (options.generate_svg, "svg")] {
        if !flag {
            continue;
        }
        let out_path = out_path.with_extension(ext);
        let filename = out_path.to_str().unwrap();
        let mut child = Command::new("dot")
            .arg(format! {"-T{}", ext})
            .args(&["-o", filename])
            .stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        child
            .stdin
            .as_mut()
            .ok_or("Child process stdin has not been captured!")
            .unwrap()
            .write_all(&dot_src.as_bytes())?;

        let output = child.wait_with_output()?;
        if !output.status.success() {
            return Err(anyhow::anyhow!("dot failed to generate {}", filename));
        }
        info!("Generated {}", filename);
    }

    Ok(())
}
