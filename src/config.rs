use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::{fs, io};

use flexstr::SharedStr;

use crate::var::Vars;
use crate::FragmentLists;

const BUF_SIZE: usize = u16::MAX as usize;

#[derive(Clone, Debug, Default, serde::Deserialize, PartialEq)]
pub struct Common {
    #[serde(default)]
    pub base_path: PathBuf,
    #[serde(default)]
    pub rustfmt_path: PathBuf,
    #[serde(default)]
    pub vars: Vars,
}

#[derive(Clone, Debug, Default, serde::Deserialize, PartialEq)]
pub struct File {
    pub path: PathBuf,
    pub fragment_list: SharedStr,
    #[serde(default)]
    pub fragment_list_exceptions: Vec<SharedStr>,
    pub vars: Vars,
}

#[derive(Clone, Debug, Default, serde::Deserialize, PartialEq)]
pub struct Config {
    #[serde(default)]
    pub common: Common,
    pub fragment_lists: FragmentLists,
    pub files: HashMap<SharedStr, File>,
}

impl Config {
    /// Try to load the config from the given reader
    pub fn from_reader(r: impl io::Read) -> anyhow::Result<Self> {
        let mut reader = io::BufReader::new(r);
        let mut buffer = String::with_capacity(BUF_SIZE);
        reader.read_to_string(&mut buffer)?;

        let mut config: Config = toml::from_str(&buffer)?;
        config.fragment_lists = config.fragment_lists.build();
        Ok(config)
    }

    pub fn from_file(cfg_name: impl AsRef<Path>) -> anyhow::Result<Config> {
        match fs::File::open(cfg_name) {
            // If the file exists, but it can't be deserialized then report that error
            Ok(f) => Ok(Self::from_reader(f)?),
            // Report any other I/O errors
            Err(err) => Err(err.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::str::FromStr;

    use flexstr::{shared_str, SharedStr};
    use pretty_assertions::assert_eq;

    use crate::config::{Common, Config, File};
    use crate::var::{CodeValue, VarItem, VarValue};
    use crate::{FragmentItem, FragmentLists};

    const CONFIG: &str = r#"
        [common]
        base_path = "src/"
        rustfmt_path = "rustfmt"
        
        [common.vars]
        product = "FlexStr"
        generate = true
        count = 5
        suffix = "$ident$Str"
        list = [ "FlexStr", true, 5, "$ident$Str" ]
                
        [fragment_lists]
        impl = [ "impl_struct", "impl_core_ref" ]
        impl_struct = [ "empty", "from_ref" ]
        
        [files.str]
        path = "strings/generated/std_str.rs"
        fragment_list = "impl"
        fragment_list_exceptions = [ "impl_core_ref" ]
        
        [files.str.vars]
        str_type = "str"
    "#;

    fn common() -> Common {
        let mut vars = HashMap::new();

        let product = VarValue::String(shared_str!("FlexStr"));
        vars.insert(shared_str!("product"), VarItem::Single(product.clone()));

        let generate = VarValue::Bool(true);
        vars.insert(shared_str!("generate"), VarItem::Single(generate.clone()));

        let count = VarValue::Number(5);
        vars.insert(shared_str!("count"), VarItem::Single(count.clone()));

        let suffix = VarValue::CodeValue(CodeValue::from_str("$ident$Str").unwrap());
        vars.insert(shared_str!("suffix"), VarItem::Single(suffix.clone()));

        vars.insert(
            shared_str!("list"),
            VarItem::List(vec![product, generate, count, suffix]),
        );

        Common {
            base_path: PathBuf::from("src/"),
            rustfmt_path: PathBuf::from("rustfmt"),
            vars,
        }
    }

    fn fragment_lists() -> FragmentLists {
        use FragmentItem::*;

        let mut lists = HashMap::new();
        lists.insert(
            shared_str!("impl"),
            vec![
                FragmentListRef(shared_str!("impl_struct")),
                Fragment(shared_str!("impl_core_ref")),
            ],
        );
        lists.insert(
            shared_str!("impl_struct"),
            vec![
                Fragment(shared_str!("empty")),
                Fragment(shared_str!("from_ref")),
            ],
        );
        FragmentLists(lists)
    }

    fn files() -> HashMap<SharedStr, File> {
        let mut str_vars = HashMap::new();
        str_vars.insert(
            shared_str!("str_type"),
            VarItem::Single(VarValue::String(shared_str!("str"))),
        );

        let files_str = File {
            path: PathBuf::from("strings/generated/std_str.rs"),
            fragment_list: shared_str!("impl"),
            fragment_list_exceptions: vec![shared_str!("impl_core_ref")],
            vars: str_vars,
        };

        let mut files = HashMap::new();
        files.insert(shared_str!("str"), files_str);
        files
    }

    #[test]
    fn from_reader() {
        let actual = Config::from_reader(CONFIG.as_bytes()).unwrap();
        let expected = Config {
            common: common(),
            fragment_lists: fragment_lists(),
            files: files(),
        };

        assert_eq!(expected, actual);
    }
}
