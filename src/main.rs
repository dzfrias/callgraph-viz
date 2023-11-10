#![allow(dead_code)]

mod generate_graph;
mod visualize;

use anyhow::Result;

fn main() -> Result<()> {
    visualize::init("./assets/scc.py");

    Ok(())
}
