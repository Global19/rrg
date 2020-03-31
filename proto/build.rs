// Copyright 2020 Google LLC
//
// Use of this source code is governed by an MIT-style license that can be found
// in the LICENSE file or at https://opensource.org/licenses/MIT.

use std::path::Path;
use std::io::{Read, Write, Result};

const PROTOS: &'static [&'static str] = &[
    "grr/grr/proto/grr_response_proto/semantic.proto",
    "grr/grr/proto/grr_response_proto/knowledge_base.proto",
    "grr/grr/proto/grr_response_proto/jobs.proto",
];

const INCLUDES: &'static [&'static str] = &[
    "grr/grr/proto",
];

fn main() {
    // Because GRR proto files are not something that PROST! can accept (because
    // of the missing package definitions), we create a temporary directory with
    // recreated file structure. We apply the patch to each of them and feed the
    // Protocol Buffers compiler with updated files.
    let tempdir = tempfile::tempdir()
        .expect("failed to create temp dir");

    let mut protos = Vec::new();
    let mut includes = Vec::new();

    for path_before in PROTOS {
        let path_after = tempdir.path().join(path_before);

        patch_path(&path_before, &path_after)
            .expect(&format!("failed to patch file '{}'", path_before));

        protos.push(path_after);
    }

    for path_before in INCLUDES {
        let path_after = tempdir.path().join(path_before);
        includes.push(path_after);
    }

    prost_build::compile_protos(&protos, &includes)
        .expect("failed to compile proto files");

    // There is also a problem with one enum generated by PROST!: it's values
    // use name mangling, but it's default value does not. This is likely a bug
    // in PROST! itself, but for now we hack around it by replacing the spurious
    // line in the output file ourselves.
    let outdir = std::env::var("OUT_DIR")
        .expect("no output directory");

    let target = Path::new(&outdir).join("grr.rs");

    let grr = std::fs::read_to_string(&target)
        .expect("invalid generated Rust code")
        .replace("TskFsAttrTypeDefault", "Default");

    std::fs::write(&target, grr)
        .expect("failed to write updated output file");
}

/// Patches given file at path `input`, writing patched content at `output`.
///
/// This function takes a path to malformed (i.e. lacking package definition)
/// GRR proto file and converts it to something that PROST! can understand. This
/// workaround has to be used as long as GRR does not fixes its proto files
/// upstream, which might be hard because of compatibility reasons.
fn patch_path<PI, PO>(input: PI, output: PO) -> Result<()>
where
    PI: AsRef<Path>,
    PO: AsRef<Path>,
{
    let mut input = file::open(&input)?;
    let mut output = file::create(&output)?;

    patch_buffer(&mut input, &mut output)
}

/// Patches given `input` buffer, writing patched content to `output`.
///
/// This function takes a buffer with malformed (lacking package definition)
/// GRR proto file and converts it to something that PROST! can understand. This
/// workaround has to be used as long as GRR does not fixes its proto files
/// upstream, which might be hard because of compatibility reasons.
fn patch_buffer<R, W>(input: &mut R, output: &mut W) -> Result<()>
where
    R: Read,
    W: Write,
{
    let mut buffer = String::new();
    input.read_to_string(&mut buffer)?;

    for line in buffer.lines() {
        writeln!(output, "{}", line)?;
        if line.starts_with("syntax =") {
            writeln!(output, "package grr;")?;
        }
    }

    Ok(())
}

mod file {
    use std::fs::File;
    use std::io::Result;
    use std::path::Path;

    /// Opens a file at the specified `path`.
    ///
    /// This function simply delegates to the standard library `File::open` and
    /// exists purely for aesthetic purposes.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<File> {
        File::open(path)
    }

    /// Creates a file at the specified `path` and all the necessary folders
    /// along the way.
    ///
    /// Except for directory creation, this function should behave identically
    /// to the standard library's `File::create`.
    pub fn create<P: AsRef<Path>>(path: P) -> Result<File> {
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)?;
        }

        File::create(path)
    }
}
