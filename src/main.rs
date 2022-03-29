use std::{
    collections::BTreeMap,
    fs::{create_dir_all, read_dir, File},
    io::{BufReader, BufWriter, Cursor, Read, Write},
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::{Context, Result};
use clap::Parser;
use message_dehash::{get_file_name, try_possible_name};
use pmd_code_table::CodeTable;
use pmd_farc::{hash_name, message_dehash, Farc, FarcWriter};
use pmd_message::MessageBin;
use translatepmd::{Entry, GettextWriter};

/// A tool that can be used to translate PSMD (US rom)
#[derive(Parser)]
struct Opts {
    #[clap(subcommand)]
    subcmd: SubCommand,
}

pub fn get_code_table(path: &Path) -> Result<CodeTable> {
    let code_table_file = File::open(path)
        .with_context(|| format!("can't open the code_table file at {:?}", path))?;
    let mut code_table =
        CodeTable::new_from_file(code_table_file).context("can't load the code_table file")?;
    code_table.add_missing();
    Ok(code_table)
}

#[derive(Clone, Copy)]
enum PoStorageMode {
    File,
    Folder,
}

impl FromStr for PoStorageMode {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, &'static str> {
        match s {
            "file" => Ok(PoStorageMode::File),
            "folder" => Ok(PoStorageMode::Folder),
            _ => Err("The mod should be either 'file' or 'folder'".into()),
        }
    }
}

impl PoStorageMode {
    pub fn write(self, path: &Path, content: GettextWriter) -> Result<()> {
        match self {
            PoStorageMode::File => {
                let pot = content.to_pot();
                let mut file = File::create(path).with_context(|| {
                    format!("can't create the POT file at {:?}", path.to_string_lossy())
                })?;
                file.write_all(pot.as_bytes()).with_context(|| {
                    format!(
                        "can't write to the POT file at {:?}",
                        path.to_string_lossy()
                    )
                })?;
                Ok(())
            }
            PoStorageMode::Folder => {
                let mut files: BTreeMap<String, GettextWriter> = BTreeMap::new();
                for entry in content.entries.into_iter() {
                    let source_file = entry.source_file.clone();
                    let specific_file = files
                        .entry(source_file)
                        .or_insert_with(|| GettextWriter::new(Vec::new()));
                    specific_file.entries.push(entry);
                }
                create_dir_all(path).with_context(|| {
                    format!(
                        "can't create the POT files folder at {:?}",
                        path.to_string_lossy()
                    )
                })?;
                for (source_file, gettext) in files.into_iter() {
                    let source_file = format!("{}.po", source_file.split(".").next().unwrap_or(""));
                    let mut file =
                        File::create(path.join(source_file.clone())).with_context(|| {
                            format!(
                                "can't create the POT file at {:?}",
                                path.join(&source_file).to_string_lossy()
                            )
                        })?;
                    file.write_all(gettext.to_pot().as_bytes())
                        .with_context(|| {
                            format!(
                                "can't write to the POT file at {:?}",
                                path.join(source_file).to_string_lossy()
                            )
                        })?;
                }
                Ok(())
            }
        }
    }
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

#[derive(Parser)]
enum SubCommand {
    ToPot(ToPotParameter),
    FromPo(FromPoParameter),
}

#[derive(Parser)]
struct ToPotParameter {
    /// The mode, can be either folder or farc
    mode: Mode,
    /// The code_table.bin file, containing information about placeholder
    code_table: PathBuf,
    /// The type of output. either file or folder
    storage_mode: PoStorageMode,
    /// The input message folder/farc file (depend on mode)
    input: PathBuf,
    /// The output pot file/folder
    output: PathBuf,
    /// The list of phrase that could have multiple different meaning
    unique: Vec<String>,
}

#[derive(Parser)]
struct FromPoParameter {
    /// The mode, can be either folder or farc
    mode: Mode,
    /// The code_table.bin file, containing information about placeholder
    code_table: PathBuf,
    input: PathBuf,
    output: PathBuf,
}

fn main() -> Result<()> {
    let opts = Opts::parse();

    match opts.subcmd {
        SubCommand::ToPot(topot_p) => topot(&topot_p).context("while creating the POT file")?,
        SubCommand::FromPo(frompo_p) => {
            frompo(&frompo_p).context("While creating the translation .bin file")?
        }
    };

    Ok(())
}

fn topot(topot_p: &ToPotParameter) -> Result<()> {
    let code_table =
        get_code_table(&topot_p.code_table).context("can't read the code_table.bin file")?;

    let mut gettext = GettextWriter::new(topot_p.unique.clone());
    let code_to_text = code_table.generate_code_to_text();
    match topot_p.mode {
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
                let mut file = BufReader::new(File::open(file_path)?);
                let message_bin = MessageBin::load_file(&mut file, Some(&code_to_text))?;
                for (hash, unk, text) in message_bin.messages().iter() {
                    gettext.entries.push(Entry::new(
                        text.clone(),
                        *hash,
                        *unk,
                        file_name_str.to_string(),
                    ));
                }
            }
        }
        Mode::Farc => {
            let farc_file = BufReader::new(
                File::open(&topot_p.input).context("can't open the input farc file")?,
            );
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

            let mut list_file = BufReader::new(File::open(list_file_path).with_context(|| {
                format!("can't open the related list file {:?}", list_file_path)
            })?);

            try_possible_name(&mut farc, &mut list_file).with_context(|| {
                format!("error reading the related list file {:?}", list_file_path)
            })?;

            for file_name in farc.iter_name() {
                let mut message_file = farc.get_named_file(file_name).with_context(|| {
                    format!("can't load the {:?} message file from the farc", file_name)
                })?;
                let message_bin = MessageBin::load_file(&mut message_file, Some(&code_to_text))?;
                for (hash, unk, text) in message_bin.messages().iter() {
                    gettext.entries.push(Entry::new(
                        text.clone(),
                        *hash,
                        *unk,
                        file_name.to_string(),
                    ));
                }
            }
        }
    }

    topot_p
        .storage_mode
        .write(&topot_p.output, gettext)
        .context("can't write the result file")?;

    Ok(())
}

fn frompo(frompo_p: &FromPoParameter) -> Result<()> {
    let mut input_file = BufReader::new(File::open(&frompo_p.input)?);
    let code_table = get_code_table(&frompo_p.code_table)?;
    let text_to_code = code_table.generate_text_to_code();

    let mut po_file = String::new();
    input_file.read_to_string(&mut po_file)?;

    let (translation, warnings) = GettextWriter::from_po(po_file);
    for warning in &warnings {
        println!("non fatal warning: {}", warning);
    }

    match frompo_p.mode {
        Mode::Folder => todo!(),
        Mode::Farc => {
            let mut translated_file: BTreeMap<String, MessageBin> = BTreeMap::new();
            for entry in translation.entries.iter() {
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
                message_bin.write(&mut buffer, Some(&text_to_code))?;
                farc_writer.add_hashed_file(hash_name(&file_name), buffer.into_inner());
            }
            let mut out_file = BufWriter::new(File::create(&frompo_p.output)?);
            farc_writer.write_hashed(&mut out_file)?;
        }
    }
    Ok(())
}
