use std::collections::HashMap;
use std::sync::Arc;

use crate::oneshot::{OneShotItem};
use crate::driver::IntoTale;
use crate::{Tale, DiskPail};

use shrine_storage::{slot, store::Namespace};


#[derive(Debug, Clone)]
pub struct DocTale {
    lede: String,
    help: String,
}

impl IntoTale for DocTale {
    fn into_tale(&self, _ns: Arc<Namespace>) -> Tale {
        let mut t = Tale::empty();
        if self.lede.len() != 0 { t.push(slot::LEDE, DiskPail::Text { data: self.lede.clone().into_bytes() }) };

        if self.help.len() != 0 { t.push(slot::HELP, DiskPail::Text { data: self.help.clone().into_bytes() }) };

        t
    }
}


impl DocTale {
    pub const fn new_(lede: String, help: String) -> Self {
        Self {
            lede,
            help,
        }
    }

    pub fn new(lede: impl Into<String>, help: impl Into<String>) -> Self {
        Self::new_(lede.into(), help.into())
    }

}

pub fn help(pax: &str, title: &str, body: &str) -> OneShotItem {
    let pax = pax.to_string();
    let title = title.to_string();
    let body = body.to_string();
    let tale = DocTale::new(title, body);
    return OneShotItem {
        path: pax,
        item: Arc::new(tale)
    }
}

#[derive(Debug)]
pub struct BuildTale(String, Option<String>);

impl IntoTale for BuildTale {
    fn into_tale(&self, namespace: Arc<Namespace>) -> Tale {
        let mut t = Tale::empty();
        let boot = vec![namespace.get_path_id("/sys/boot").unwrap()];
        t.push(slot::KOOK, DiskPail::Duct { data: boot });
        t.push(slot::SRC, DiskPail::Text { data: self.0.clone().into_bytes() });
        if let Some(sut) = self.1.as_ref() {
            let sut = namespace.get_path_id(sut).unwrap();
            let mut deps = HashMap::new();
            deps.insert("sut".to_string(), sut);
            t.push(slot::CREW, DiskPail::Mesh { data: deps })
            // let crew = DiskPail::Mesh { data: "sut" }
            // t.push()
        }

        t
    }
}

pub fn build(path: &str, txt: &str) -> OneShotItem {
    OneShotItem{
        path: path.to_string(),
        item: Arc::new(BuildTale(txt.to_string(), None))
    }
}

pub fn build_sut(path: &str, txt: &str, sut: &str) -> OneShotItem {
    OneShotItem{
        path: path.to_string(),
        item: Arc::new(BuildTale(txt.to_string(), Some(sut.to_string())))
    }
}

pub const SYS_DOCS_STR: &'static str = r#"
The /sys subtree contains all of the vine and dirt core functionality. It \
should not typically be used in day to day use of the. Rebuilding any of the
components here will produce a rebuild of the entire system. Additionally,
the slots required for the vine and dirt to bootstrap themselves are also
located here
"#;

pub const PARSE_LOON: &'static str = include_str!("../parse-loon.hoon");
pub const FORD_LIB: &'static str = include_str!("../ford.hoon");


pub fn get_docs() -> Vec<OneShotItem> {
    vec![
    help("/neo", "Standard Distribution", ""),
    build("/neo/lib/ford", FORD_LIB),
    build_sut("/neo/kok/parse-loon", PARSE_LOON, "/neo/lib/ford"),
    help("/sys/slot/help", "Help", ""),
    help("/sys/slot/lede", "Title", ""),
    help("/sys/slot/clan", "Children Schema", r#"
The clan slot is used to define a schema for the children of a particular shrub.
Usually, this is occupied by a $soma. If a $soma is present at this slot, then
the vine will assert that any new shrubs that are parents of the shrub in question
match the soma, aborting the event if they do not. Otherwise, the contents of the
clan slot are purely advisory. Usually, the $soma will be a computed slot, being
derived from the imps slot.
"#),
    help("/sys/slot/vase", "Vase", r#"
Vases are used ubiquitiously in shrubbery kernelspace for metaprogramming.
Their use in userspace is planned to be phased out, however, while they remain
 you should be careful. There are checks on so-called 'evil' vases, but this
can be bypassed if you use the %vase stud
"#),
    help("/sys/slot/unreads", "Unreads", "Used to indicate that a path has unreads")
    ]
}
