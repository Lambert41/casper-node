use std::{
    fs,
    path::{Path, PathBuf},
    process,
};

use clap::{App, Arg, ArgMatches, SubCommand};
use lazy_static::lazy_static;

use casperlabs_node::crypto::asymmetric_key::{PublicKey, SecretKey};

use crate::{common, Subcommand as CrateSubcommand};

const ACCOUNT_ID_BASE64: &str = "account_id_base64";
const ACCOUNT_ID_HEX: &str = "account_id_hex";
const PUBLIC_KEY_BASE64: &str = "public_key_base64";
const PUBLIC_KEY_HEX: &str = "public_key_hex";
const SECRET_KEY_PEM: &str = "secret_key.pem";
const PUBLIC_KEY_PEM: &str = "public_key.pem";
const FILES: [&str; 6] = [
    ACCOUNT_ID_BASE64,
    ACCOUNT_ID_HEX,
    PUBLIC_KEY_BASE64,
    PUBLIC_KEY_HEX,
    SECRET_KEY_PEM,
    PUBLIC_KEY_PEM,
];

lazy_static! {
    static ref MORE_ABOUT: String = format!("{}. Creates {:?}", Keygen::ABOUT, FILES);
}

/// This struct defines the order in which the args are shown for this subcommand's help message.
enum DisplayOrder {
    OutputDir,
    Force,
    Algorithm,
}

/// Handles providing the arg for and retrieval of the output directory.
mod output_dir {
    use super::*;

    const ARG_NAME: &str = "output-dir";
    const ARG_VALUE_NAME: &str = "PATH";
    const ARG_HELP: &str =
        "Path to output directory where key files will be created. If the path doesn't exist, it \
        will be created. If not set, the current working directory will be used";

    pub(super) fn arg() -> Arg<'static, 'static> {
        Arg::with_name(ARG_NAME)
            .required(false)
            .value_name(ARG_VALUE_NAME)
            .help(ARG_HELP)
            .display_order(DisplayOrder::OutputDir as usize)
    }

    pub(super) fn get(matches: &ArgMatches) -> PathBuf {
        common::string_to_path_buf(matches.value_of(ARG_NAME).unwrap_or_else(|| "."))
    }
}

/// Handles the arg for whether to overwrite existing output files.
mod force {
    use super::*;

    pub(super) const ARG_NAME: &str = "force";
    const ARG_NAME_SHORT: &str = "f";
    const ARG_HELP: &str =
        "If this flag is passed, any existing output files will be overwritten. Without this flag, \
        if any output file exists, no output files will be generated";

    pub(super) fn arg() -> Arg<'static, 'static> {
        Arg::with_name(ARG_NAME)
            .short(ARG_NAME_SHORT)
            .required(false)
            .help(ARG_HELP)
            .display_order(DisplayOrder::Force as usize)
    }

    pub(super) fn get(matches: &ArgMatches) -> bool {
        matches.is_present(ARG_NAME)
    }
}

/// Handles providing the arg for and retrieval of the node hostname/IP and port.
mod algorithm {
    use super::*;

    const ARG_NAME: &str = "algorithm";
    const ARG_SHORT: &str = "a";
    const ARG_VALUE_NAME: &str = "STRING";
    pub(super) const ED25519: &str = "Ed25519";
    pub(super) const SECP256K1: &str = "secp256k1";
    const ARG_HELP: &str = "The type of keys to generate";

    pub fn arg() -> Arg<'static, 'static> {
        Arg::with_name(ARG_NAME)
            .long(ARG_NAME)
            .short(ARG_SHORT)
            .required(false)
            .default_value(ED25519)
            .possible_value(ED25519)
            .possible_value(SECP256K1)
            .value_name(ARG_VALUE_NAME)
            .help(ARG_HELP)
            .display_order(DisplayOrder::Algorithm as usize)
    }

    pub fn get(matches: &ArgMatches) -> String {
        matches
            .value_of(ARG_NAME)
            .unwrap_or_else(|| panic!("should have {} arg", ARG_NAME))
            .to_string()
    }
}

pub struct Keygen {}

impl<'a, 'b> crate::Subcommand<'a, 'b> for Keygen {
    const NAME: &'static str = "keygen";
    const ABOUT: &'static str = "Generates account key files in the given directory";

    fn build(display_order: usize) -> App<'a, 'b> {
        SubCommand::with_name(Self::NAME)
            .about(MORE_ABOUT.as_str())
            .display_order(display_order)
            .arg(output_dir::arg())
            .arg(force::arg())
            .arg(algorithm::arg())
    }

    fn run(matches: &ArgMatches<'_>) {
        let output_dir = output_dir::get(matches);
        let force = force::get(matches);
        let algorithm = algorithm::get(matches);

        let _ = fs::create_dir_all(&output_dir)
            .unwrap_or_else(|error| panic!("should create {}: {}", output_dir.display(), error));
        let output_dir = output_dir.canonicalize().expect("should canonicalize path");

        if !force {
            for file in FILES.iter().map(|filename| output_dir.join(filename)) {
                if file.exists() {
                    eprintln!(
                        "{} exists. To overwrite, rerun with --{}",
                        file.display(),
                        force::ARG_NAME
                    );
                    process::exit(1);
                }
            }
        }

        let secret_key = if algorithm == algorithm::ED25519 {
            SecretKey::generate_ed25519()
        } else if algorithm == algorithm::SECP256K1 {
            SecretKey::generate_secp256k1()
        } else {
            panic!("Invalid key algorithm");
        };
        let public_key = PublicKey::from(&secret_key);
        let account_id = public_key.to_account_hash().value();

        write_file(
            ACCOUNT_ID_BASE64,
            output_dir.as_path(),
            base64::encode(&account_id),
        );
        write_file(
            ACCOUNT_ID_HEX,
            output_dir.as_path(),
            hex::encode(&account_id),
        );
        write_file(
            PUBLIC_KEY_BASE64,
            output_dir.as_path(),
            base64::encode(public_key.as_ref()),
        );
        write_file(
            PUBLIC_KEY_HEX,
            output_dir.as_path(),
            hex::encode(public_key.as_ref()),
        );

        let secret_key_path = output_dir.join(SECRET_KEY_PEM);
        secret_key
            .to_file(&secret_key_path)
            .unwrap_or_else(|error| {
                panic!("should write {}: {}", secret_key_path.display(), error)
            });

        let public_key_path = output_dir.join(PUBLIC_KEY_PEM);
        public_key
            .to_file(&public_key_path)
            .unwrap_or_else(|error| {
                panic!("should write {}: {}", public_key_path.display(), error)
            });

        println!("Wrote files to {}", output_dir.display());
    }
}

fn write_file(filename: &str, dir: &Path, value: String) {
    let path = dir.join(filename);
    fs::write(&path, value)
        .unwrap_or_else(|error| panic!("should write {}: {}", path.display(), error))
}
