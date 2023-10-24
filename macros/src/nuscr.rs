

extern crate proc_macro;
use proc_macro::TokenStream;
use std::fs::File;
use std::path::Path;
use std::{convert::TryFrom, io::Write};
use litrs::StringLit;
use std::process::Command;

static COMPILE_SCRIPT: &str = r#"#!/bin/sh

GENERATE="$(which rumpsteak-generate || echo $2)"

FILE=$1
PROTO="$(nuscr --enum $FILE | sed -n -e "/@/ s/.*@//p" | head -1)"

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
			sed "s/<.*>//" > ${endpoint}.dot || failwith "Can not generate .dot files (nuscr error)."
	done
}

dot2rs() {
	DOT="$(echo ${ENDPOINTS}.dot | sed s/\ /.dot\ /g)"

	$GENERATE --name $PROTO $DOT || failwith "Can not generate .rs file (rumpsteak-generate error)."
}

scan_endpoints
nuscr2dot
dot2rs"#;

pub fn generate_nuscr_rs(protocol_file: TokenStream) -> TokenStream {
    let protocol_file_str = match StringLit::try_from(protocol_file.into_iter().next().unwrap()) {
        // Error if the token is not a string literal
        Err(e) => return e.to_compile_error(),
        Ok(lit) => lit,
    };
    let protocol_file_str = protocol_file_str.value();

    let mut temp_compile_script = tempfile::NamedTempFile::new().unwrap();
    temp_compile_script.write_all(COMPILE_SCRIPT.as_bytes()).unwrap();
    temp_compile_script.flush().unwrap();

    let protocol_path = {
        let span = proc_macro::Span::call_site();
        let source = span.source_file();
        let source_path = source.path();
        let folder = source_path.parent().unwrap();
        Path::join(folder, protocol_file_str)
    };
        
        let compile_script_path = temp_compile_script.path().to_string_lossy().to_string();
        let mut file = File::create("foo.txt").unwrap();
        // let stdin = std::process::Stdio::piped();

        let child = Command::new("sh")
            .arg(compile_script_path)
            .arg(protocol_path.to_str().unwrap())
            .arg("./target/debug/rumpsteak-generate")
            .output()
            .expect("compile script command failed to start");

        // let mut stdin_write = child.stdin.take().expect("Failed to open stdin");
        // stdin_write.write_all(inline_compile_script.as_bytes());

        // file.write_all(Command::new("sh")
        // .arg("-c")
        // .arg(inline_compile_script)
        // .arg(protocol_file_str)
        // .arg("rumpsteak-generate")
        // .get_args().map(|arg| arg.to_string_lossy().to_string()).collect::<Vec<_>>().join(",,").as_bytes()).unwrap();
        file.write_all(&child.stderr).unwrap();
        file.write_all(&child.stdout).unwrap();
        String::from_utf8(child.stdout).unwrap().parse().unwrap()
    // (quote::quote! {
        
    //     run_command_str!("sh", "-c", std::stringify! (inline_compile_script), #protocol_file_str).parse().unwrap();
    // }).into()
}