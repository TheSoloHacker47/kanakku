//! What goes on a share image, for each kind of page.

use kanakku_core::flags::FlagKind;
use kanakku_core::fmt::inr_short;
use kanakku_core::i18n::{flag_label, Lang};
use kanakku_core::kiifb_status::FundedProject;
use kanakku_core::model::{Headline, Project};
use kanakku_core::title::display_title;

use crate::db::ProjectPage;
use crate::og::Card;

/// A card's text, owned, so it can be built from a record and then lent to the renderer.
pub struct Owned {
    eyebrow: String,
    title: String,
    figure: String,
    figure_label: String,
    share: Option<f32>,
    flag: Option<String>,
}

impl Owned {
    pub fn borrow(&self) -> Card<'_> {
        Card {
            eyebrow: &self.eyebrow,
            title: &self.title,
            figure: &self.figure,
            figure_label: &self.figure_label,
            share: self.share,
            flag: self.flag.as_deref(),
        }
    }
}

pub fn project(project: &Project, data: &ProjectPage) -> Owned {
    let place = if project.district.is_empty() { "Kerala".to_string() } else { format!("{} district", project.district) };
    let (figure, figure_label, share) = match project.headline() {
        Some((amount, Headline::Estimate)) => {
            let spent = project.expenditure.filter(|_| amount > 0);
            let label = match spent {
                Some(spent) => format!("estimated · {} spent so far", inr_short(spent, Lang::En)),
                None => "estimated amount".to_string(),
            };
            (inr_short(amount, Lang::En), label, spent.map(|spent| spent as f32 / amount as f32))
        }
        Some((amount, Headline::Spent)) => (inr_short(amount, Lang::En), "spent on this package so far".to_string(), None),
        Some((amount, Headline::WorksTotal)) => (inr_short(amount, Lang::En), "total sanctioned for its works".to_string(), None),
        None => ("No amount published".to_string(), "as KIIFB reports it".to_string(), None),
    };
    let open: Vec<FlagKind> = data.flags.iter().filter(|f| f.status == "open").filter_map(|f| FlagKind::parse(&f.kind)).collect();
    let flag = open.first().map(|kind| match open.len() {
        1 => flag_label(Lang::En, *kind).to_string(),
        n => format!("{} +{}", flag_label(Lang::En, *kind), n - 1),
    });
    Owned {
        eyebrow: format!("{} · {place} · KIIFB project", project.code),
        title: display_title(&project.title),
        figure,
        figure_label,
        share,
        flag,
    }
}

pub fn funded(project: &FundedProject) -> Owned {
    let (figure, figure_label, share) = match (project.paid(), project.approved) {
        (Some(paid), Some(approved)) if approved > 0 => (
            inr_short(paid, Lang::En),
            format!("paid of {} approved by KIIFB", inr_short(approved, Lang::En)),
            (paid <= approved).then_some(paid as f32 / approved as f32),
        ),
        (Some(paid), _) => (inr_short(paid, Lang::En), "paid so far; not yet approved in full".to_string(), None),
        _ => ("Under evaluation".to_string(), "not approved yet".to_string(), None),
    };
    Owned {
        eyebrow: format!("KIIFB project status · {}", project.department.as_deref().unwrap_or("Kerala")),
        title: display_title(&project.name),
        figure,
        figure_label,
        share,
        flag: None,
    }
}
