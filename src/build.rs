use std::env;
use std::fs;
use std::fs::DirEntry;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use tempfile::NamedTempFile;

fn create_temp_compile_script() -> NamedTempFile {
    static COMPILE_SCRIPT: &str = r#"#!/bin/sh

    GENERATE="cargo bin rumpsteak-generate"
    
    FILE=$1
    PROTO="$(nuscr --enum $FILE | sed -n -e "/@/ s/.*@//p" | head -1)"
    TEMPDIR=$3

    failwith() {
        echo [FAIL] $1 1>&2
        exit 127
    }
    
    scan_endpoints() {
        ENDPOINTS="$(nuscr --enum $FILE | sed -n -e "/@/ s/@.*//p")"
    }
    
    nuscr2dot() {
        for endpoint in $ENDPOINTS; do
            nuscr --fsm $endpoint@$PROTO ${FILE} |
                sed "s/digraph G/digraph $endpoint/" |
                sed s/int/i32/ |
                sed "s/<.*>//" > ${TEMPDIR}/${endpoint}.dot || failwith "Can not generate .dot files (nuscr error)."
        done
    }
    
    dot2rs() {
        REPLACE_PATTERN='s:([[:alnum:]]+\.dot):'$TEMPDIR'/\1:g'
        DOT="$(echo ${ENDPOINTS}.dot | sed s/\ /.dot\ /g | sed -E ${REPLACE_PATTERN})"
    
        $GENERATE --name $PROTO $DOT || failwith "Can not generate .rs file (rumpsteak-generate error)."
    }
    
    scan_endpoints
    nuscr2dot
    dot2rs"#;

    let mut temp_compile_script = tempfile::NamedTempFile::new().unwrap();
    temp_compile_script.write_all(COMPILE_SCRIPT.as_bytes()).unwrap();
    temp_compile_script.flush().unwrap();

    temp_compile_script
}

pub fn create_protocols_module(module_filepath: PathBuf, included_protocols: Vec<String>) {
    let contents = {
        let module_lines: Vec<String> = included_protocols.iter().map(|protocol_name| format!("pub mod {};", protocol_name)).collect();
        module_lines.join("\n")
    };
    fs::write(module_filepath, contents).unwrap()
}

pub fn compile_nuscr_protocols() {
    let root = env::var_os("CARGO_MANIFEST_DIR").unwrap();
    let protocol_folder = Path::new(&root).join("src/protocols");
    let nuscr_source_folder = protocol_folder.join("nuscr");
    println!("cargo:rerun-if-changed={}", protocol_folder.to_str().unwrap());
    let protocols = fs::read_dir(nuscr_source_folder).expect("Couldn't find src/protocols/nuscr/ folder in root");

    let compile_script = create_temp_compile_script ();

    let temp_dir = tempfile::tempdir().unwrap();

    let compiled_protocols: Vec<String> = protocols.filter_map(|file| {
        let file = file.unwrap();
        let protocol_path = DirEntry::path(&file);
        if !file.file_name().to_str().unwrap().ends_with(".nuscr") {return None}

        let child = Command::new("sh")
        .arg(compile_script.path())
        .arg(protocol_path.to_str().unwrap())
        .arg("./target/debug/rumpsteak-generate")
        .arg(temp_dir.path().to_str().unwrap())
        .output()
        .expect("compile script command failed to start");

        let generated_file_name = Path::file_stem(&protocol_path).unwrap().to_str().unwrap();
        // if !generated_file_name.ends_with(".nuscr") {
        //     return
        // };
        let target_generated_path = protocol_folder.join(format!("{}.rs", generated_file_name));


        let mut file = File::create(target_generated_path).unwrap();
        file.write_all(&child.stderr).unwrap();
        file.write_all(&child.stdout).unwrap();

        println!("STDOUT (rumpsteak-build-gen): {}", String::from_utf8(child.stdout).unwrap());
        println!("STDERR (rumpsteak-build-gen): {}", String::from_utf8(child.stderr).unwrap());

        Some(generated_file_name.to_owned())
}).collect();

    create_protocols_module(protocol_folder.join("mod.rs"), compiled_protocols);

    temp_dir.close().unwrap();

    println!("cargo:rerun-if-changed=build.rs");
}