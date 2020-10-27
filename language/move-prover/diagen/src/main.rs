// Copyright (c) The Libra Core Contributors
// SPDX-License-Identifier: Apache-2.0

use diagen::{run_diagram_generator, DiagenOptions};
use env_logger::Env;
use log::{error, info};
use std::{env, fs, io, path::PathBuf};

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    if let Ok(lang_dir) = locate_language_dir() {
        info!("The `language` directory is located as:\n{:?}\n", lang_dir);

        // construct the input and the output paths
        let inp_dir = lang_dir.join("stdlib/modules");
        let out_dir = lang_dir.join("move-prover/diagen/diagrams");

        if let Err(err) = run_diagram_generator(
            &move_files_in(&inp_dir),
            &out_dir,
            &DiagenOptions {
                generate_dot: true,
                generate_global_graph: true,
                ..Default::default()
            },
        ) {
            eprintln!("Error: {:?}", err);
        } else {
            println!(
                "To convert the generated .dot files into .pdf files, run {:?}.",
                out_dir.join("convert_all_dot_to_pdf.sh")
            );
        }
    } else {
        error!("Error: Cannot locate the language directory.");
    }
}

pub fn move_files_in(path: &PathBuf) -> Vec<PathBuf> {
    fs::read_dir(path)
        .unwrap()
        .filter_map(|dir_entry| {
            let path = dir_entry.unwrap().path();
            if path.is_file() && path.extension().is_some() && path.extension().unwrap() == "move" {
                Some(path)
            } else {
                None
            }
        })
        .collect()
}

// Locate the path of the "language" directory in the Libra repository.
pub fn locate_language_dir() -> io::Result<PathBuf> {
    Ok(env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("language"))
}
