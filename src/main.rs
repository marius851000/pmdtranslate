use std::{
    collections::BTreeMap,
    fs::{read_dir, File},
    io::{Cursor, Read, Write},
    path::PathBuf,
    str::FromStr,
};

use anyhow::{Context, Result};
use clap::Clap;
use message_dehash::{get_file_name, try_possible_name};
use pmd_farc::{hash_name, message_dehash, Farc, FarcWriter};
use pmd_message::MessageBin;
use translatepmd::{Entry, GettextWriter};

/// A tool that can be used to translate PSMD (US rom)
#[derive(Clap)]
struct Opts {
    /// The mode, can be either folder or farc
    mode: Mode,
    #[clap(subcommand)]
    subcmd: SubCommand,
}

enum Mode {
    Farc,
    Folder,
}

impl FromStr for Mode {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "farc" => Mode::Farc,
            "folder" => Mode::Folder,
            _ => return Err("the mode parameter should be either \"farc\" or \"folder\""),
        })
    }
}

#[derive(Clap)]
enum SubCommand {
    ToPot(ToPotParameter),
    FromPo(FromPoParameter),
}

#[derive(Clap)]
struct ToPotParameter {
    /// The input message folder/farc file (depend on mode)
    input: PathBuf,
    /// The output pot file
    output: PathBuf,
}

#[derive(Clap)]
struct FromPoParameter {
    input: PathBuf,
    output: PathBuf,
}

fn main() -> Result<()> {
    let opts = Opts::parse();

    match opts.subcmd {
        SubCommand::ToPot(topot_p) => topot(opts.mode, topot_p)?,
        SubCommand::FromPo(frompo_p) => frompo(opts.mode, frompo_p)?,
    };

    Ok(())
}

/*fn main() {
    frompo(Mode::Farc, FromPoParameter {
        input: "out.pot".into(),
        output: "./message_us_custom.bin".into()
    }).unwrap();
    for (source_name, out_name) in &[("message_us_custom.bin", "1_custom.bin"), ("message_us.bin", "1_original.bin")] {
        let file = File::open(source_name).unwrap();
        let farc = Farc::new(file).unwrap();
        let mut first_file = farc.get_named_file("kaichuuclear1st.bin").unwrap();
        let mut out_file = File::create(out_name).unwrap();
        std::io::copy(&mut first_file, &mut out_file).unwrap();
    };
}*/

fn topot(mode: Mode, topot_p: ToPotParameter) -> Result<()> {
    let mut gettext = GettextWriter::new();
    match mode {
        Mode::Folder => {
            for file_maybe in
                read_dir(&topot_p.input).context("can't list files in the input directory")?
            {
                let file_entry = file_maybe
                    .context("can't get data about one element of the input directory")?;
                let file_name = file_entry.file_name();
                if file_name == "name_sort.bin" {
                    continue;
                };
                let file_name_str = file_name
                    .to_str()
                    .context("can't transform the file name to an utf8 string")?;
                let file_path = topot_p.input.join(file_entry.file_name());
                let mut file = File::open(file_path)?;
                let message_bin = MessageBin::load_file(&mut file)?;
                for (hash, unk, text) in message_bin.messages().iter() {
                    gettext.entrys.push(Entry::new(
                        text.clone(),
                        *hash,
                        *unk,
                        file_name_str.to_string(),
                    ));
                }
            }
        }
        Mode::Farc => {
            let farc_file = File::open(&topot_p.input).context("can't open the input farc file")?;
            let mut farc = Farc::new(farc_file)?;
            let list_file_name = get_file_name(
                topot_p
                    .input
                    .file_name()
                    .unwrap()
                    .to_str()
                    .context("the input file name isn't a valid utf8 file")?,
            )
            .context("can't get the associated list file")?;

            let list_file_path = &topot_p
                .input
                .parent()
                .map(|x| x.to_path_buf())
                .unwrap_or(PathBuf::from("."))
                .join(&list_file_name);

            let mut list_file = File::open(list_file_path).with_context(|| {
                format!("can't open the related list file {:?}", list_file_path)
            })?;

            try_possible_name(&mut farc, &mut list_file).with_context(|| {
                format!("error reading the related list file {:?}", list_file_path)
            })?;

            for file_name in farc.iter_name() {
                let mut message_file = farc.get_named_file(file_name).with_context(|| {
                    format!("can't load the {:?} message file from the farc", file_name)
                })?;
                let message_bin = MessageBin::load_file(&mut message_file)?;
                for (hash, unk, text) in message_bin.messages().iter() {
                    gettext.entrys.push(Entry::new(
                        text.clone(),
                        *hash,
                        *unk,
                        file_name.to_string(),
                    ));
                }
            }
        }
    }

    let mut out_file = File::create(&topot_p.output).context("can't open the output file")?;
    out_file.write_all(gettext.to_pot().as_bytes())?;

    Ok(())
}

fn frompo(mode: Mode, frompo_p: FromPoParameter) -> Result<()> {
    let mut input_file = File::open(&frompo_p.input)?;
    let mut po_file = String::new();
    input_file.read_to_string(&mut po_file)?;

    let (translation, warnings) = GettextWriter::from_po(po_file);
    for warning in &warnings {
        println!("non fatal warning: {}", warning);
    }

    match mode {
        Mode::Folder => todo!(),
        Mode::Farc => {
            let mut translated_file: BTreeMap<String, MessageBin> = BTreeMap::new();
            for entry in translation.entrys.iter() {
                let message_bin = if let Some(entry) = translated_file.get_mut(&entry.source_file) {
                    entry
                } else {
                    translated_file.insert(entry.source_file.clone(), MessageBin::default());
                    translated_file.get_mut(&entry.source_file).unwrap()
                };
                message_bin.insert(entry.hash, entry.unk, entry.text.clone());
            }
            let mut farc_writer = FarcWriter::default();
            for (file_name, message_bin) in translated_file {
                let mut buffer = Cursor::new(Vec::new());
                message_bin.write(&mut buffer)?;
                farc_writer.add_hashed_file(hash_name(&file_name), buffer.into_inner());
            }
            let mut out_file = File::create(&frompo_p.output)?;
            farc_writer.write_hashed(&mut out_file)?;
        }
    }
    Ok(())
}
