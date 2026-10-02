//! The standing pages: methodology, open data, map and not-found.

use kanakku_core::flags::{FlagKind, RULES_VERSION};
use kanakku_core::i18n::{flag_label, flag_rule, Lang};
use kanakku_core::stage::Stage;
use maud::{html, Markup};

use super::{en, icon, layout, Nav, Page};
use crate::db::Status;
use crate::icons;
use crate::runs::source_name;

fn head(lang: Lang, title: &str, lead: &str) -> Markup {
    html! {
        div.phead {
            div.wrap {
                nav.crumbs aria-label=(lang.pick("വഴി", "Breadcrumb")) {
                    a href=(format!("{}/", lang.prefix())) { (lang.t().site_name) }
                    (icon(icons::CHEVRON_RIGHT))
                    span { (title) }
                }
                h1.d2 { (title) }
                p.lead { (lead) }
            }
        }
    }
}

pub fn methodology(lang: Lang, origin: &str, last_checked: Option<&str>) -> String {
    let t = lang.t();
    let lead = lang.pick(
        "സർക്കാർ തന്നെ പ്രസിദ്ധീകരിക്കുന്ന പദ്ധതി വിവരങ്ങൾ ശേഖരിച്ച്, വായിക്കാവുന്ന രൂപത്തിൽ, ഉറവിടത്തോടൊപ്പം കാണിക്കുന്നു. അതു മാത്രം.",
        "We collect the project figures the government itself publishes and show them in a readable form, next to their source. Nothing more.",
    );
    let body = html! {
        (head(lang, t.methodology_title, lead))
        div.wrap {
            div.prose {
                h2 { (lang.pick("തത്വങ്ങൾ", "Principles")) }
                @match lang {
                    Lang::Ml => ul {
                        li { "ഓരോ സംഖ്യയും അതിന്റെ ഉറവിടത്തിലേക്ക് ചൂണ്ടുന്നു. ഞങ്ങൾ ശേഖരിച്ച താളിന്റെ പകർപ്പ് സൂക്ഷിക്കുന്നു; അത് ആർക്കും ഡൗൺലോഡ് ചെയ്യാം." }
                        li { "വസ്തുതകളും സൂചനകളും മാത്രം. “14 മാസം വൈകി” എന്ന് പറയും; കാരണം ഊഹിക്കില്ല, ആരെയും കുറ്റപ്പെടുത്തില്ല." }
                        li { "എല്ലാ കക്ഷികൾക്കും എം.എൽ.എമാർക്കും തദ്ദേശ സ്ഥാപനങ്ങൾക്കും ഒരേ നിയമങ്ങൾ." }
                    },
                    Lang::En => ul {
                        li { "Every number points to its source. We keep a copy of the page we collected it from, and anyone can download that copy." }
                        li { "Facts and flags only. We say “14 months past scheduled completion”; we do not guess at reasons or blame anyone." }
                        li { "The same rules apply to every party, MLA and local body." }
                    },
                }

                h2 #rules { (lang.pick("സൂചനാ നിയമങ്ങൾ", "Flag rules")) " · " (t.rule_version) " " (RULES_VERSION) }
                p { (lang.pick(
                    "ഒരു സൂചന എന്നത് പ്രസിദ്ധീകരിച്ച സംഖ്യകൾ ഒരു പരിധി കടന്നു എന്നതിന്റെ അടയാളം മാത്രമാണ്. അത് തെറ്റ് നടന്നു എന്നതിന്റെ തെളിവല്ല. ഓരോ സൂചനയിലും അതിന് ആധാരമായ സംഖ്യകളും നിയമത്തിന്റെ പതിപ്പും കാണാം.",
                    "A flag only marks that published figures crossed a threshold. It is not evidence of wrongdoing. Each flag shows the figures it rests on and the version of the rule that raised it.",
                )) }
                div.rules {
                    @for kind in FlagKind::ALL {
                        div {
                            @let href = if kind == FlagKind::PaidAboveApproval {
                                format!("{}/funding?flag=1", lang.prefix())
                            } else {
                                format!("{}/projects?flag={}", lang.prefix(), kind.as_str())
                            };
                            a.badge.flag href=(href) { (icon(icons::FLAG)) (flag_label(lang, kind)) }
                            p { (flag_rule(lang, kind)) }
                        }
                    }
                }

                h2 #stages { (lang.pick("ഘട്ടങ്ങൾ", "Stages")) }
                p { (lang.pick(
                    "കിഫ്ബിയുടെ സ്ഥിതിവിവരങ്ങൾ (“WBS Base One Approved” പോലുള്ളവ) അവരുടെ ആഭ്യന്തര പ്രവർത്തനക്രമത്തിലെ പദങ്ങളാണ്. വായിക്കാൻ എളുപ്പത്തിന് ഞങ്ങൾ അവയെ താഴെയുള്ള ഘട്ടങ്ങളാക്കി തിരിക്കുന്നു. ഈ തരംതിരിവ് ഞങ്ങളുടേതാണ്; കിഫ്ബിയുടെ യഥാർത്ഥ പദം ഓരോ പദ്ധതിയുടെയും താളിൽ കാണിക്കുന്നു.",
                    "KIIFB's status labels, such as “WBS Base One Approved”, are terms from its internal workflow. To make them readable we group them into the stages below. The grouping is ours; each project page also shows KIIFB's original label.",
                )) }
                p { (lang.pick(
                    "കിഫ്ബി ഈ ക്രമം എവിടെയും പൂർണ്ണമായി വിശദീകരിക്കുന്നില്ല. കിഫ്ബിയുടെ റോഡ് പ്രവൃത്തി മാർഗ്ഗനിർദ്ദേശം (2018) അനുസരിച്ച് പ്രോജക്ട് എക്സിക്യൂഷൻ ഡോക്യുമെന്റ് സാങ്കേതികാനുമതിക്കു ശേഷമാണ്; അതിനാൽ അത് സാങ്കേതികാനുമതിയുടെ ഘട്ടത്തിൽ വരുന്നു. “WBS Base Zero”, “Base One” എന്നിവയുടെ അർത്ഥം കിഫ്ബി പ്രസിദ്ധീകരിച്ചിട്ടില്ല. Base One ഉള്ള പദ്ധതികളിൽ മിക്കവയിലും ചെലവ് തുടങ്ങിയിട്ടുണ്ട്; Base Zero ഉള്ളവയിൽ ഒന്നിലും ഇല്ല. രണ്ടും ഞങ്ങൾ പ്രവൃത്തിപദ്ധതി ഘട്ടത്തിൽ വയ്ക്കുന്നു.",
                    "KIIFB does not set out this order in full anywhere. Its guidelines for road works (2018) place the Project Execution Document after technical sanction, so we put it in that stage. KIIFB has not published what “WBS Base Zero” and “Base One” mean. Most projects at Base One have started spending; none at Base Zero has. We put both in the work plan stage.",
                )) }
                div.scroll tabindex="0" role="region" aria-label=(lang.pick("പട്ടിക: വശങ്ങളിലേക്ക് നീക്കാം", "Table, scrolls sideways")) {
                    table {
                        thead { tr { th scope="col" { (lang.pick("ഘട്ടം", "Stage")) } th scope="col" { (lang.pick("കിഫ്ബിയുടെ പദങ്ങൾ", "KIIFB labels containing")) } } }
                        tbody {
                            @for stage in Stage::ALL {
                                tr {
                                    td { b { (stage.label(lang)) } }
                                    td lang="en" {
                                        @match stage {
                                            Stage::Preparation => "Project Initiated, Project Created, Design Basis Report, Submitted for Confirmation",
                                            Stage::TechnicalSanction => "Technical Sanction, Project Execution Document",
                                            Stage::Tender => "Tender",
                                            Stage::Contract => "Contract Process",
                                            Stage::WorkPlan => "WBS, Payment milestone",
                                            Stage::Returned => "Return to SPV",
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                h2 { (lang.pick("ഡാറ്റ എവിടെ നിന്ന്", "Where the data comes from")) }
                p { (lang.pick(
                    "കിഫ്ബിയുടെ സംയോജിത ഡാഷ്ബോർഡ് (gis.kiifb.org) ദിവസത്തിൽ ഒരിക്കൽ ഞങ്ങൾ വായിക്കുന്നു. ഉള്ളടക്കം മാറിയിട്ടുണ്ടെങ്കിൽ മാത്രം പുതിയ പകർപ്പ് സൂക്ഷിക്കും; മാറിയ ഓരോ വിവരവും പദ്ധതിയുടെ താളിൽ രേഖപ്പെടുത്തും.",
                    "We read KIIFB's integrated dashboard (gis.kiifb.org) once a day. We store a new copy only when its contents have changed, and every field that changed is recorded on the project's page.",
                )) }

                p { (lang.pick(
                    "കിഫ്ബിയുടെ പ്രോജക്ട് സ്റ്റാറ്റസ് താളും (kiifb.org/prjStatus.jsp) ദിവസവും വായിക്കുന്നു. അതിൽ ഓരോ പദ്ധതിക്കും അനുവദിച്ച തുകയും നൽകിയ തുകയും, ഓരോ പ്രവൃത്തിക്കും അനുവദിച്ചതും നൽകിയതും ഉണ്ട്. പല ജില്ലകളിലായുള്ള പദ്ധതികൾക്ക് കിഫ്ബിയുടെ പട്ടികയിലെ “നൽകിയ തുക” അതിലെ പ്രവൃത്തികളുടെ ആകെത്തുകയുടെ കൃത്യമായ ഗുണിതമാണ് (രണ്ടിരട്ടി, നാലിരട്ടി, ഒരിടത്ത് പതിനാലിരട്ടി): ഒരേ തുക ഓരോ ജില്ലയ്ക്കും വീണ്ടും കൂട്ടുന്നു. അതിനാൽ നൽകിയ തുക ഞങ്ങൾ പ്രവൃത്തികളിൽ നിന്നാണ് എടുക്കുന്നത്; രണ്ടും വ്യത്യസ്തമായിടത്ത് കിഫ്ബിയുടെ സംഖ്യയും കൂടെ കാണിക്കുന്നു. തുക മാറിയ പദ്ധതികളുടെ പ്രവൃത്തിപ്പട്ടിക അന്നുതന്നെ വീണ്ടും വായിക്കും; മറ്റുള്ളവ ഊഴമനുസരിച്ച്, ഓരോ രാത്രിയും കുറച്ചെണ്ണം വീതം.",
                    "We also read KIIFB's project status page (kiifb.org/prjStatus.jsp) once a day. It gives, for each project, the amount approved and the amount released, and the same for each work. For a project filed under several districts, the “released” figure on KIIFB's list is an exact multiple of what its works add up to (twice, four times, in one case fourteen times): the list counts the same payments once per district. We therefore take what has been paid from the works, and show KIIFB's listed figure beside it wherever the two differ. When a project's figures move we re-read its work table the same night; the rest are re-read in turn, a few each night.",
                )) }

                p { (lang.pick(
                    "കേരള പൊതുമരാമത്ത് വകുപ്പിന്റെ ഡി.എൽ.പി (തകരാർ ബാധ്യതാ കാലാവധി) പട്ടിക ആഴ്ചയിലൊരിക്കൽ വായിക്കുന്നു. ഒരു പ്രവൃത്തിയുടെ ജില്ല എന്നത് അത് കൈകാര്യം ചെയ്യുന്ന പി.ഡബ്ല്യു.ഡി ഓഫീസിന്റെ പേരിൽ നിന്നെടുത്ത ജില്ലയാണ്; ചില ഓഫീസുകൾ അയൽജില്ലകളിലെ പ്രവൃത്തികളും നോക്കുന്നു. ആ താളുകളിൽ കരാറുകാരുടെയും ഉദ്യോഗസ്ഥരുടെയും ഫോൺ നമ്പറുകളുണ്ട്; ഞങ്ങൾ അവ സൂക്ഷിക്കുന്നില്ല. അതുകൊണ്ട് ഈ ഉറവിടത്തിന് താളിന്റെ പകർപ്പല്ല, ഞങ്ങൾ വായിച്ച വരികളുടെ പകർപ്പാണ് സൂക്ഷിക്കുന്നത്. ഓരോ വരിയുടെയും കോഡിൽ കരാർ തുകയുമുണ്ട്; വകുപ്പിന്റെ താളിൽ അത് കാണിക്കുന്നില്ല. ഞങ്ങൾ അത് വായിച്ച് കാണിക്കുന്നു, അക്കാര്യം താളിൽ പറയുന്നു. അതേ രീതിയിൽ മറച്ചിട്ടുള്ള കരാറുകാരന്റെ വിലാസം ഞങ്ങൾ വായിക്കുന്നില്ല. വകുപ്പ് പ്രവൃത്തികൾക്ക് തിരിച്ചറിയൽ നമ്പർ നൽകുന്നില്ല; പട്ടികയിൽ നിന്ന് ഒരു പ്രവൃത്തി ഒഴിവായാൽ ഞങ്ങൾ അത് രേഖപ്പെടുത്തും. വകുപ്പിന്റെ പട്ടികയിൽ ചില പ്രവൃത്തികൾ ഒന്നിലധികം തവണ വരുന്നുണ്ട്; പേരും തുടക്കത്തീയതിയും കരാറുകാരനും ഒന്നായവ ഞങ്ങൾ ഒന്നായി എണ്ണുന്നു. അതിനാൽ ഞങ്ങളുടെ എണ്ണം വകുപ്പിന്റേതിനേക്കാൾ കുറവായിരിക്കും.",
                    "We read the Kerala Public Works Department's DLP (defect liability period) list once a week. A work's district is that of the PWD office handling it, taken from the office's name; some offices also handle works in neighbouring districts. Those pages print contractors' and officers' phone numbers; we do not keep them. So for this source the stored copy is our extract of the rows, not the page itself. Each row's markup also carries the agreed contract amount, commented out so that PWD's page does not display it; we read and show it, and say so on the page. The contractor's address, hidden the same way, we do not read. PWD gives works no identifier; when a work drops off the list we record that. PWD's list repeats some works; rows with the same name, start date and contractor are counted once here, so our count is lower than PWD's.",
                )) }

                h2 #join { (lang.pick("രണ്ട് ഉറവിടങ്ങൾ ചേർക്കുന്നത്", "Joining the two sources")) }
                p { (lang.pick(
                    "കിഫ്ബിയുടെ ഈ രണ്ട് താളുകൾക്കും പൊതുവായ തിരിച്ചറിയൽ നമ്പറില്ല. സ്റ്റാറ്റസ് താളിലെ “പദ്ധതി” ഡാഷ്ബോർഡിലെ ഒരു ഉപപദ്ധതിയാണ്; അതിലെ “പ്രവൃത്തികൾ” ഡാഷ്ബോർഡിലെ കരാർ പാക്കേജുകളും. അനുവദിച്ച തുക (രൂപ വരെ കൃത്യമായി), വകുപ്പ്, നിർവഹണ സ്ഥാപനം എന്നിവ മൂന്നും ഒന്നാണെങ്കിൽ, അങ്ങനെ യോജിക്കുന്നത് ഒന്നു മാത്രമാണെങ്കിൽ, ഞങ്ങൾ അവ ചേർക്കുന്നു. സ്ഥാപനത്തിന്റെ പേര് വ്യത്യസ്തമാണെങ്കിൽ പേരുകളിലെ വാക്കുകൾ ഭൂരിഭാഗവും യോജിക്കണം. ഒന്നിലധികം സാധ്യതകളുണ്ടെങ്കിൽ ചേർക്കില്ല. പ്രവൃത്തികളെ പാക്കേജുകളുമായി ചേർക്കുന്നത് പ്രസിദ്ധീകരിച്ച പേര് അതേപടി ഒന്നാണെങ്കിൽ മാത്രം.",
                    "KIIFB's two pages share no identifier. A “project” on the status page is a sub-project on the dashboard, and its “works” are the dashboard's contract packages. We join them when the approved amount (to the rupee), the department and the implementing agency all agree and only one record fits. Where the agency is named differently, most of the words in the names must agree. Where more than one record could fit, we do not join. A work is joined to a package only when the published titles are identical.",
                )) }
                p { (lang.pick(
                    "ഒരു പ്രവൃത്തിക്ക് അനുവദിച്ചതിനേക്കാൾ 1 ശതമാനത്തിലധികം കൂടുതൽ നൽകിയതായി കിഫ്ബി കാണിക്കുന്നിടത്ത് ഞങ്ങൾ അത് എടുത്തുപറയുന്നു. അതിലും ചെറിയ വ്യത്യാസങ്ങൾ പൈസയുടെ കണക്കാണ്. ഇത് തെറ്റ് നടന്നതിന്റെ തെളിവല്ല; പുതുക്കിയ അനുമതി താളിൽ വരാത്തതാകാം.",
                    "Where KIIFB shows a work paid more than 1% above what was approved for it, we say so. Smaller differences are paise and rounding. This is not evidence of wrongdoing; a revised approval may simply not be shown on the page.",
                )) }

                h2 { (lang.pick("പരിമിതികൾ", "Limitations")) }
                @match lang {
                    Lang::Ml => ul {
                        li { "തുകകൾ ഡാഷ്ബോർഡിൽ ഉള്ളതുപോലെയാണ്. പൂജ്യം എന്ന് കാണിച്ചവ “പ്രസിദ്ധീകരിച്ചിട്ടില്ല” എന്ന് ഞങ്ങൾ കണക്കാക്കുന്നു." }
                        li { "കണക്കാക്കിയ തുക കിഫ്ബി നൽകുന്നത് ഉപപദ്ധതിയുടെ തലത്തിലാണ്. ഒരു ഉപപദ്ധതിക്ക് കീഴിൽ പല കരാർ പാക്കേജുകളുണ്ടെങ്കിൽ എല്ലാറ്റിലും അതേ തുക ആവർത്തിക്കും; അങ്ങനെയുള്ളിടത്ത് ഞങ്ങൾ അത് വ്യക്തമാക്കുന്നു, തുകകൾ കൂട്ടുന്നില്ല." }
                        li { "കരാറുകാരൻ, തീയതികൾ, പുരോഗതി എന്നിവ റോഡ്, പാലം പ്രവൃത്തികൾക്ക് മാത്രമേ ഡാഷ്ബോർഡിൽ ഉള്ളൂ." }
                        li { "ഭൂപടത്തിലെ സ്ഥാനങ്ങൾ കിഫ്ബി നൽകിയവയാണ്; ചിലത് തെറ്റായിരിക്കാം." }
                        li { "സ്റ്റാറ്റസ് താളിലെ ചില പദ്ധതികൾ പല ജില്ലകളിലായുള്ളവയാണ്; അതിനാൽ ഒരു ജില്ലയുടെ ആകെത്തുക ആ ജില്ലയ്ക്ക് മാത്രമുള്ളതല്ല. ഒൻപതിൽ ഒന്നോളം പദ്ധതികൾക്ക് കിഫ്ബി ജില്ല രേഖപ്പെടുത്തിയിട്ടില്ല; അവ സംസ്ഥാനത്തിന്റെ ആകെത്തുകയിൽ മാത്രം വരും. ഡാഷ്ബോർഡിലെ “ചെലവും” സ്റ്റാറ്റസ് താളിലെ “നൽകിയ തുകയും” വ്യത്യസ്ത സംഖ്യകളാണ്; രണ്ടും അതത് ഉറവിടത്തിൽ ഉള്ളതുപോലെ കാണിക്കുന്നു." }
                        li { "“ചെലവ് വർധന”, “പുതിയ വിവരമില്ല” എന്നീ സൂചനകൾ ഞങ്ങൾ ശേഖരണം തുടങ്ങിയ ശേഷമുള്ള മാറ്റങ്ങളെ മാത്രം അടിസ്ഥാനമാക്കിയാണ്." }
                        li { "പദ്ധതികളുടെ പേരുകൾ കിഫ്ബി ഇംഗ്ലീഷിലാണ് പ്രസിദ്ധീകരിക്കുന്നത്. തലക്കെട്ടിൽ ഞങ്ങൾ ഫയൽ കോഡുകൾ നീക്കുകയും വലിയക്ഷരങ്ങൾ സാധാരണ രൂപത്തിലാക്കുകയും ചെയ്യുന്നു; യഥാർത്ഥ പേര് താളിൽ കാണാം." }
                        li { "മണ്ഡലങ്ങളുടെ പേരുകൾ കിഫ്ബി പല രീതിയിൽ എഴുതുന്നു (Vypeen, Vypin, Vyppin). ഞങ്ങൾ അവ ഒന്നാക്കുന്നു. എം.എൽ.എമാരുടെ പേരുകൾ കിഫ്ബി പട്ടികയിൽ ഉള്ളതുപോലെയാണ്." }
                    },
                    Lang::En => ul {
                        li { "Amounts are as the dashboard states them. Where it shows zero, we treat the figure as not published." }
                        li { "KIIFB states the estimated amount per sub-project. Where several contract packages sit under one sub-project, each repeats the same estimate; we say so on the page and never add those estimates up." }
                        li { "The dashboard gives contractor, dates and progress only for road and bridge works." }
                        li { "Map locations are the ones KIIFB records; some may be wrong." }
                        li { "Some projects on the status page span several districts, so a district's totals are not that district's alone. KIIFB files about one project in nine under no district at all; those appear only in the state's totals. The dashboard's “expenditure” and the status page's “released” are different figures; we show each as its source states it." }
                        li { "“Cost escalation” and “No recent update” rely only on changes since we began collecting." }
                        li { "KIIFB publishes project names in English. In headlines we drop filing codes and calm all-capital titles; the name as published is shown on the page." }
                        li { "KIIFB spells constituencies several ways (Vypeen, Vypin, Vyppin). We merge them. MLA names are as KIIFB lists them." }
                    },
                }

                h2 { (lang.pick("തിരുത്തലുകൾ", "Corrections")) }
                p { (lang.pick(
                    "ഞങ്ങൾ കാണിക്കുന്നത് ഉറവിടത്തിൽ നിന്ന് വ്യത്യസ്തമാണെങ്കിൽ അറിയിക്കുക. ഉറവിടവുമായി ഒത്തുനോക്കി ഞങ്ങൾ തിരുത്തും.",
                    "If what we show differs from the source, tell us. We will check it against the source and correct it.",
                )) }
                p { a href=(format!("{}/about#corrections", lang.prefix())) { (lang.pick("എങ്ങനെ അറിയിക്കാം", "How to tell us")) } }
            }
        }
    };
    layout(&page(lang, t.methodology_title, lead, "/methodology", origin, Nav::Methodology, last_checked), body)
}

pub fn data(lang: Lang, origin: &str, last_checked: Option<&str>) -> String {
    let t = lang.t();
    let lead = lang.pick(
        "ഈ സൈറ്റിലെ എല്ലാ വിവരങ്ങളും യന്ത്രങ്ങൾക്ക് വായിക്കാവുന്ന രൂപത്തിൽ ലഭ്യമാണ്. ലോഗിൻ വേണ്ട.",
        "Everything on this site is available in machine-readable form. No login is needed.",
    );
    let endpoints: [(&str, &str, &str, &str); 7] = [
        ("/api/v1/projects.csv", "CSV", "സ്പ്രെഡ്ഷീറ്റിൽ തുറക്കാൻ: എല്ലാ പദ്ധതികളും", "All projects, for spreadsheets"),
        ("/api/v1/projects", "JSON", "എല്ലാ പദ്ധതികളും, പ്രവൃത്തികൾ ഉൾപ്പെടെ", "All projects, with their works"),
        ("/api/v1/projects.geojson", "GeoJSON", "പദ്ധതി സ്ഥാനങ്ങൾ", "Project locations"),
        ("/api/v1/funding.csv", "CSV", "അനുവദിച്ചതും നൽകിയതും: ഓരോ പ്രവൃത്തിയും ഒരു വരി", "Approved and paid, one row per work"),
        ("/api/v1/funding", "JSON", "അനുവദിച്ചതും നൽകിയതും, പ്രവൃത്തികൾ ഉൾപ്പെടെ", "Approved and paid, with works"),
        ("/api/v1/liability.csv", "CSV", "കരാറുകാരന്റെ ബാധ്യതാ കാലാവധിയിലുള്ള പി.ഡബ്ല്യു.ഡി പ്രവൃത്തികൾ", "PWD works under contractor liability"),
        ("/api/v1/liability", "JSON", "ബാധ്യതാ കാലാവധിയിലുള്ള പ്രവൃത്തികൾ", "Works under contractor liability"),
    ];
    // Paged endpoints: what they return, and an example address.
    let paged: [(&str, &str, &str, &str); 3] = [
        ("/api/v1/changes", "/api/v1/changes?since=2026-10-01", "ഞങ്ങൾ രേഖപ്പെടുത്തിയ ഓരോ മാറ്റവും, പുതിയത് ആദ്യം", "Every change we recorded, newest first"),
        ("/api/v1/flags", "/api/v1/flags?status=all&type=overdue", "സൂചനകൾ: നിലവിലുള്ളവയും നീക്കിയവയും", "Flags, open and cleared"),
        ("/api/v1/snapshots", "/api/v1/snapshots?source=1", "ഓരോ സംഖ്യയ്ക്കും ആധാരമായ, ഞങ്ങൾ സൂക്ഷിച്ച ഉറവിട പകർപ്പുകൾ", "The stored source copies every figure traces back to"),
    ];
    let body = html! {
        (head(lang, t.data_title, lead))
        div.wrap {
            section.sec {
                div.tiles {
                    @for (path, kind, ml, en) in endpoints {
                        a.tile href=(path) {
                            (icon(icons::DOWNLOAD))
                            b { (kind) }
                            span.small.muted { (lang.pick(ml, en)) }
                            span.code { (path) }
                        }
                    }
                }
            }
            div.prose {
                h2 { (lang.pick("ഒരു പദ്ധതി മാത്രം", "One project")) }
                p {
                    span.code { "/api/v1/projects/{code}" } " "
                    (lang.pick(
                        "ഒരു പദ്ധതിയുടെ രേഖ, സൂചനകൾ, മാറ്റങ്ങൾ, ഉറവിടം എന്നിവ നൽകുന്നു.",
                        "returns one project's record, flags, recorded changes and source.",
                    ))
                }
                h2 { (lang.pick("പേജുകളായി", "In pages")) }
                p { (lang.pick(
                    "ഇവ 500 വരികൾ വീതമുള്ള പേജുകളായി നൽകുന്നു; “next” അടുത്ത പേജിന്റെ വിലാസമാണ്. district= ഒരു ജില്ലയിലേക്ക് ചുരുക്കുന്നു.",
                    "These come 500 rows to a page; “next” is the address of the next page. district= narrows to one district.",
                )) }
                ul {
                    @for (path, example, ml, en) in paged {
                        li { a href=(example) { span.code { (path) } } " " (lang.pick(ml, en)) }
                    }
                }
                p {
                    (lang.pick("എല്ലാ വിലാസങ്ങളുടെയും ഫീൽഡുകളുടെയും പൂർണ്ണ വിവരണം (OpenAPI): ", "A full description of every address and field (OpenAPI): "))
                    a href="/api/v1/openapi.json" { span.code { "/api/v1/openapi.json" } }
                }
                h2 { (lang.pick("ഉപയോഗിക്കുമ്പോൾ", "Using the data")) }
                ul {
                    li { (lang.pick("തുകകൾ മുഴുവൻ രൂപയിലാണ്. തീയതികൾ yyyy-mm-dd രൂപത്തിൽ.", "Amounts are whole rupees. Dates are yyyy-mm-dd.")) }
                    li { (lang.pick(
                        "“estimated_amount” ഉപപദ്ധതിയുടേതാണ്; “estimate_shared_by” ഒന്നിൽ കൂടുതലാണെങ്കിൽ അത് കൂട്ടരുത്.",
                        "“estimated_amount” belongs to the sub-project; do not sum it where “estimate_shared_by” is greater than one.",
                    )) }
                    li { (lang.pick(
                        "“funding” ഫയലുകൾ കിഫ്ബിയുടെ പ്രോജക്ട് സ്റ്റാറ്റസ് താളിൽ നിന്നാണ്. “map_group” ഉണ്ടെങ്കിൽ അത് ഞങ്ങൾ ചേർത്ത ഡാഷ്ബോർഡ് ഉപപദ്ധതിയാണ്; ചേർത്തത് ഞങ്ങളാണ്, കിഫ്ബിയല്ല.",
                        "The “funding” files come from KIIFB's project status page. Where “map_group” is set, it is the dashboard sub-project we joined it to; the join is ours, not KIIFB's.",
                    )) }
                    li { (lang.pick(
                        "പതിപ്പ് 1-ൽ ഫീൽഡുകൾ കൂട്ടിച്ചേർക്കുകയേ ഉള്ളൂ; പേരു മാറ്റുകയോ നീക്കുകയോ ഇല്ല.",
                        "Version 1 only grows: fields are added, never renamed or removed.",
                    )) }
                }
                h2 #license { (lang.pick("ലൈസൻസ്", "Licence")) }
                p {
                    (lang.pick(
                        "സംഖ്യകൾ അവ പ്രസിദ്ധീകരിച്ച ഉറവിടങ്ങളുടേതാണ് (കിഫ്ബി, പി.ഡബ്ല്യു.ഡി). കണക്ക് കൂട്ടിച്ചേർക്കുന്നവ, അതായത് സമാഹരണം, വൃത്തിയാക്കിയ പേരുകൾ, ഘട്ടങ്ങൾ, സൂചനകൾ, ഉറവിടങ്ങൾ തമ്മിലുള്ള ബന്ധിപ്പിക്കൽ എന്നിവ ",
                        "The figures belong to the sources that publish them (KIIFB, PWD). What Kanakku adds, namely the compilation, cleaned names, stages, flags and the joins between sources, is licensed ",
                    ))
                    a href="https://creativecommons.org/licenses/by/4.0/" rel="license noopener" { "CC BY 4.0" }
                    (lang.pick(
                        " ലൈസൻസിലാണ്. ആർക്കും ഉപയോഗിക്കാം, പങ്കുവയ്ക്കാം, മാറ്റം വരുത്താം; ഉറവിടവും കണക്കും പരാമർശിക്കണം എന്നു മാത്രം. ഉദാഹരണം: “ഉറവിടം: കിഫ്ബി; സമാഹരണം: കണക്ക്”.",
                        ". Anyone may use, share and adapt it, as long as they credit the source and Kanakku, for example “Source: KIIFB, compiled by Kanakku”.",
                    ))
                }
                p { (lang.pick(
                    "സൈറ്റിന്റെ സോഴ്സ് കോഡ് AGPL-3.0 ലൈസൻസിലാണ്.",
                    "The site's source code is licensed AGPL-3.0.",
                )) }
            }
        }
    };
    layout(&page(lang, t.data_title, lead, "/data", origin, Nav::Data, last_checked), body)
}

pub fn map(lang: Lang, origin: &str, last_checked: Option<&str>) -> String {
    let t = lang.t();
    let head_markup = html! {
        link rel="stylesheet" href="/vendor/leaflet.css";
        script defer src="/vendor/leaflet.js" {}
        script defer src="/vendor/protomaps-leaflet.js" {}
        script defer src="/map.js" {}
    };
    let body = html! {
        (head(lang, t.map_title, t.map_intro))
        div.wrap {
            div #map data-lang=(lang.code()) data-prefix=(lang.prefix()) { p.small { (t.map_loading) } }
            p.legend {
                span { i.dot.f {} (t.map_legend_flagged) }
                span { i.dot {} (t.map_legend_clear) }
                a.more href=(format!("{}/projects", lang.prefix())) { (lang.pick("പട്ടികയായി കാണുക", "See as a list")) (icon(icons::ARROW_RIGHT)) }
            }
            p.small.muted { (lang.pick(
                "ഭൂപടത്തിലെ കുത്തുകൾ തുറക്കാൻ മൗസോ സ്പർശമോ വേണം. കീബോർഡോ സ്ക്രീൻ റീഡറോ ഉപയോഗിക്കുന്നവർക്ക് ഇതേ പദ്ധതികളെല്ലാം പട്ടികയിൽ ലഭ്യമാണ്.",
                "Opening a pin on the map needs a mouse or touch. If you use a keyboard or a screen reader, every project shown here is also in the list.",
            )) }
        }
    };
    let mut p = page(lang, t.map_title, t.map_intro, "/map", origin, Nav::Map, last_checked);
    p.head = head_markup;
    layout(&p, body)
}

/// Whether the nightly reads are working, and how much the site is used.
pub fn status(lang: Lang, origin: &str, status: &Status, now_ms: i64) -> String {
    let title = lang.pick("പ്രവർത്തന നില", "Status");
    let lead = lang.pick(
        "ഓരോ ഉറവിടവും അവസാനം വായിച്ചത് എപ്പോൾ, രാത്രിയിലെ ശേഖരണം നടക്കുന്നുണ്ടോ, സൈറ്റ് എത്ര പേർ ഉപയോഗിക്കുന്നു.",
        "When each source was last read, whether the nightly collection is running, and how much the site is used.",
    );
    let ago = |hours: i64| match lang {
        Lang::Ml if hours < 48 => format!("{hours} മണിക്കൂർ മുമ്പ്"),
        Lang::Ml => format!("{} ദിവസം മുമ്പ്", hours / 24),
        Lang::En if hours < 48 => format!("{hours} hours ago"),
        Lang::En => format!("{} days ago", hours / 24),
    };
    let day = |at: &str| kanakku_core::Date::from_utc_timestamp_ist(at).map(kanakku_core::Date::to_dmy).unwrap_or_default();
    let body = html! {
        (head(lang, title, lead))
        div.wrap {
            section.sec {
                h2.h { (lang.pick("ഉറവിടങ്ങൾ", "Sources")) }
                div.scroll tabindex="0" role="region" aria-label=(lang.pick("പട്ടിക: വശങ്ങളിലേക്ക് നീക്കാം", "Table, scrolls sideways")) {
                    table {
                        thead { tr {
                            th scope="col" { (lang.pick("ഉറവിടം", "Source")) }
                            th scope="col" { (lang.pick("അവസാനം വിജയകരമായി വായിച്ചത്", "Last good read")) }
                            th scope="col" { (lang.pick("നില", "State")) }
                        } }
                        tbody {
                            @for source in &status.sources {
                                @let (age, stale) = crate::api::freshness(source, now_ms);
                                @let last = source.last_ok_at.as_deref().or(source.last_scraped_at.as_deref());
                                tr {
                                    td { a href=(source.base_url) rel="noopener" lang="en" { (source.name) } }
                                    td {
                                        @match (last, age) {
                                            (Some(at), Some(hours)) => { (day(at)) " · " (ago(hours)) },
                                            _ => (lang.pick("ഇതുവരെ വായിച്ചിട്ടില്ല", "Not read yet")),
                                        }
                                    }
                                    td {
                                        @if stale {
                                            span.badge.flag { (lang.pick("പഴകി", "Stale")) }
                                        } @else {
                                            span.badge { (lang.pick("പുതിയത്", "Up to date")) }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                p.small.muted {
                    (lang.pick(
                        "ഡാഷ്ബോർഡും സ്റ്റാറ്റസ് താളും ദിവസവും, പി.ഡബ്ല്യു.ഡി പട്ടിക ആഴ്ചയിലൊരിക്കലും വായിക്കുന്നു. അതിലും വൈകിയാൽ “പഴകി” എന്ന് കാണിക്കും.",
                        "The dashboard and the status page are read daily and the PWD list weekly. A source that falls behind that is shown as stale.",
                    ))
                    " "
                    a href="/api/v1/status" { (lang.pick("യന്ത്രങ്ങൾക്കുള്ള പരിശോധന", "Machine-readable check")) }
                }
            }

            @if !status.runs.is_empty() {
                section.sec {
                    h2.h { (lang.pick("സമീപകാല ശേഖരണങ്ങൾ", "Recent runs")) }
                    div.scroll tabindex="0" role="region" aria-label=(lang.pick("പട്ടിക: വശങ്ങളിലേക്ക് നീക്കാം", "Table, scrolls sideways")) {
                        table {
                            thead { tr {
                                th scope="col" { (lang.pick("തീയതി", "Date")) }
                                th scope="col" { (lang.pick("ഉറവിടം", "Source")) }
                                th scope="col" { (lang.pick("ഫലം", "Result")) }
                            } }
                            tbody {
                                @for run in &status.runs {
                                    tr {
                                        td { (day(&run.finished_at)) }
                                        td lang="en" { (source_name(run.source_id)) @if run.trigger == "manual" { " · " (lang.pick("കൈകൊണ്ട്", "by hand")) } }
                                        td {
                                            @if run.ok == 1 {
                                                (lang.pick("വിജയിച്ചു", "Succeeded"))
                                            } @else {
                                                span.badge.flag { (lang.pick("പരാജയപ്പെട്ടു", "Failed")) }
                                                @if let Some(why) = &run.summary { " " span.small { (en(&why.chars().take(160).collect::<String>())) } }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            section.sec {
                h2.h { (lang.pick("സന്ദർശനങ്ങൾ", "Visits")) }
                @if status.views.is_empty() {
                    p.muted { (lang.pick("ഇതുവരെ കണക്കില്ല.", "Nothing counted yet.")) }
                } @else {
                    div.scroll tabindex="0" role="region" aria-label=(lang.pick("പട്ടിക: വശങ്ങളിലേക്ക് നീക്കാം", "Table, scrolls sideways")) {
                        table {
                            thead { tr {
                                th scope="col" { (lang.pick("താൾ", "Page")) }
                                th scope="col" { (lang.pick("കഴിഞ്ഞ 7 ദിവസം", "Last 7 days")) }
                                th scope="col" { (lang.pick("കഴിഞ്ഞ 30 ദിവസം", "Last 30 days")) }
                            } }
                            tbody {
                                @for view in &status.views {
                                    tr { td { (view_kind_label(lang, &view.kind)) } td { (view.week) } td { (view.month) } }
                                }
                            }
                        }
                    }
                }
                @if !status.referrers.is_empty() || !status.tags.is_empty() {
                    h3 { (lang.pick("വായനക്കാർ എവിടെ നിന്ന്", "Where readers come from")) }
                    div.scroll tabindex="0" role="region" aria-label=(lang.pick("പട്ടിക: വശങ്ങളിലേക്ക് നീക്കാം", "Table, scrolls sideways")) {
                        table {
                            thead { tr {
                                th scope="col" { (lang.pick("സൈറ്റ് അല്ലെങ്കിൽ ലിങ്ക്", "Site or link")) }
                                th scope="col" { (lang.pick("കഴിഞ്ഞ 7 ദിവസം", "Last 7 days")) }
                                th scope="col" { (lang.pick("കഴിഞ്ഞ 30 ദിവസം", "Last 30 days")) }
                            } }
                            tbody {
                                @for r in &status.referrers {
                                    tr { td lang="en" { (r.key) } td { (r.week) } td { (r.month) } }
                                }
                                @for t in &status.tags {
                                    tr { td { (lang.pick("പങ്കിട്ട ലിങ്ക്: ", "Shared link: ")) span lang="en" { (t.key) } } td { (t.week) } td { (t.month) } }
                                }
                            }
                        }
                    }
                }
                p.small.muted { (lang.pick(
                    "ഓരോ തരം താളും ഓരോ ദിവസം എത്ര തവണ തുറന്നു, വായനക്കാർ ഏത് സൈറ്റിൽ നിന്ന് വന്നു (സൈറ്റിന്റെ പേര് മാത്രം, മുഴുവൻ വിലാസമല്ല), ഞങ്ങൾ പങ്കിട്ട ലിങ്കുകളിലെ ടാഗുകൾ, ഓരോ പദ്ധതിത്താളും എത്ര തവണ വായിച്ചു എന്നിവ മാത്രം ഞങ്ങൾ എണ്ണുന്നു. കുക്കികളില്ല, ഐ.പി വിലാസങ്ങൾ സൂക്ഷിക്കുന്നില്ല, ആരെയും തിരിച്ചറിയുന്നില്ല. അറിയപ്പെടുന്ന യന്ത്രങ്ങളെ ഒഴിവാക്കുന്നു. ഒരു തിരച്ചിലിൽ ഒന്നും കിട്ടിയില്ലെങ്കിൽ, തിരച്ചിൽ മെച്ചപ്പെടുത്താൻ ആ വാക്കുകളും അവയുടെ എണ്ണവും മാത്രം സൂക്ഷിക്കുന്നു; ആര് തിരഞ്ഞു എന്നില്ല.",
                    "We count only how often each kind of page is opened each day, which site readers came from (its name only, never the full address), tags on links we share, and how often each project page is read. No cookies, no stored addresses, nothing that identifies anyone. Known robots are left out. When a search finds nothing, we keep the words and a count, to improve the search; not who searched.",
                )) }
            }
        }
    };
    layout(&page(lang, title, lead, "/status", origin, Nav::None, None), body)
}

/// How to reach us, as the `CONTACT` setting gives it: an email address or a web address.
fn contact_link(lang: Lang, contact: &str) -> Markup {
    let contact = contact.trim();
    html! {
        @if contact.contains('@') && !contact.contains('/') {
            a href=(format!("mailto:{contact}")) { (contact) }
        } @else if contact.starts_with("https://") {
            a href=(contact) rel="noopener" { (contact) }
        } @else {
            (lang.pick("(ബന്ധപ്പെടാനുള്ള വിലാസം ഉടൻ ഇവിടെ ചേർക്കും)", "(a contact address will be added here shortly)"))
        }
    }
}

/// Who runs the site, how to get a mistake corrected or personal data removed, and what is kept about visitors.
pub fn about(lang: Lang, origin: &str, contact: &str) -> String {
    let title = lang.pick("കണക്കിനെക്കുറിച്ച്", "About Kanakku");
    let lead = lang.pick(
        "ആര് നടത്തുന്നു, തെറ്റുകൾ എങ്ങനെ തിരുത്തുന്നു, എന്തെല്ലാം ഞങ്ങൾ സൂക്ഷിക്കുന്നു.",
        "Who runs it, how mistakes are corrected, and what we keep.",
    );
    let data = format!("{}/data#license", lang.prefix());
    let body = html! {
        (head(lang, title, lead))
        div.wrap {
            div.prose {
                h2 #who { (lang.pick("ആര് നടത്തുന്നു", "Who runs it")) }
                p { (lang.pick(
                    "കണക്ക് കേരളത്തിലെ ഒരു സ്വതന്ത്ര പൗര സംരംഭമാണ്. കിഫ്ബി, സർക്കാർ, ഏതെങ്കിലും രാഷ്ട്രീയ കക്ഷി എന്നിവയുമായി ബന്ധമില്ല; അവരിൽ നിന്ന് പണമോ നിർദ്ദേശമോ സ്വീകരിക്കുന്നില്ല. ഇപ്പോൾ ഇത് നടത്തുന്നയാളുടെ സ്വന്തം ചെലവിലാണ് പ്രവർത്തിക്കുന്നത്. പരസ്യങ്ങളില്ല.",
                    "Kanakku is an independent civic project from Kerala. It is not affiliated with KIIFB, the government or any political party, and takes no money or direction from them. It is currently paid for by the person who runs it. There are no advertisements.",
                )) }
                p { (lang.pick(
                    "ഈ സൈറ്റ് എന്തു ചെയ്യുന്നു എന്നതിന് ഒരു നിയമമേയുള്ളൂ: സർക്കാർ പ്രസിദ്ധീകരിക്കുന്ന സംഖ്യകൾ ഉറവിടത്തോടൊപ്പം കാണിക്കുക, എല്ലാവർക്കും ഒരേ നിയമങ്ങൾ ഉപയോഗിക്കുക. എങ്ങനെയെന്ന് “രീതി” താളിൽ വിശദീകരിക്കുന്നു.",
                    "The site has one rule for what it does: show the figures the government publishes, next to their source, with the same rules for everyone. The method page explains how.",
                )) }
                p { a href=(format!("{}/methodology", lang.prefix())) { (lang.t().nav_methodology) } }

                h2 #corrections { (lang.pick("തിരുത്തലുകൾ", "Corrections")) }
                p { (lang.pick(
                    "ഇവിടെ കാണിക്കുന്ന ഒരു സംഖ്യയോ പേരോ തീയതിയോ ഉറവിടത്തിൽ നിന്ന് വ്യത്യസ്തമാണെങ്കിൽ, അല്ലെങ്കിൽ ഒരു സൂചന തെറ്റായാണ് വന്നതെന്ന് തോന്നുന്നെങ്കിൽ, ഇവിടെ എഴുതുക: ",
                    "If a figure, name or date here differs from its source, or a flag looks wrong, write to: ",
                )) (contact_link(lang, contact)) }
                @match lang {
                    Lang::Ml => ul {
                        li { "താളിന്റെ വിലാസവും എന്താണ് തെറ്റെന്നും, സാധിക്കുമെങ്കിൽ ശരിയായ വിവരത്തിന്റെ ഉറവിടവും ചേർക്കുക." }
                        li { "ഏഴു ദിവസത്തിനകം മറുപടി നൽകും." }
                        li { "തെറ്റ് ഞങ്ങളുടെ വായനയിലോ കണക്കുകൂട്ടലിലോ ആണെങ്കിൽ, ഉറവിടവുമായി ഒത്തുനോക്കി ഏഴു ദിവസത്തിനകം തിരുത്തും. ഒരു സൂചന തെറ്റായി വന്നതാണെങ്കിൽ അത് നീക്കും, അക്കാര്യം പദ്ധതിയുടെ താളിൽ രേഖപ്പെടുത്തും." }
                        li { "തെറ്റ് ഉറവിടത്തിൽ തന്നെയാണെങ്കിൽ (ഉദാഹരണത്തിന് കിഫ്ബിയുടെ ഡാഷ്ബോർഡിൽ), ഞങ്ങൾക്ക് അത് മാറ്റാനാവില്ല; അക്കാര്യം നിങ്ങളെ അറിയിക്കും, പ്രസിദ്ധീകരിച്ച സ്ഥാപനത്തെ സമീപിക്കാം." }
                    },
                    Lang::En => ul {
                        li { "Include the page's address, what is wrong and, if you can, where the correct figure is published." }
                        li { "We reply within seven days." }
                        li { "If the mistake is in how we read or calculated something, we check it against the source and correct it within seven days. A flag raised in error is removed, and the removal is recorded on the project's page." }
                        li { "If the figure is wrong at the source itself (on KIIFB's dashboard, say), we cannot change it. We will tell you so, and you can raise it with the body that published it." }
                    },
                }

                h2 #takedown { (lang.pick("വിവരങ്ങൾ നീക്കാനുള്ള അപേക്ഷ", "Requests to remove information")) }
                p { (lang.pick(
                    "പൊതുപണം ചെലവഴിക്കുന്നതിനെക്കുറിച്ച് സർക്കാർ സ്ഥാപനങ്ങൾ പ്രസിദ്ധീകരിച്ച രേഖകൾ ഈ സൈറ്റിന്റെ ഉള്ളടക്കമാണ്: പദ്ധതികൾ, തുകകൾ, തീയതികൾ, കരാറുകാരുടെ പേരുകൾ, ജനപ്രതിനിധികളുടെ പേരുകൾ. ഇവ അസൗകര്യകരമാണെന്ന കാരണത്താൽ മാത്രം നീക്കില്ല. ഉറവിടത്തിൽ തിരുത്തിയാൽ ഇവിടെയും അടുത്ത വായനയിൽ മാറും.",
                    "This site is made of records government bodies publish about public spending: projects, amounts, dates, contractors' names and the names of elected representatives. We do not remove them because they are inconvenient. When the source corrects them, they change here at the next read.",
                )) }
                p { (lang.pick(
                    "സ്വകാര്യ വ്യക്തികളുടെ ഫോൺ നമ്പറുകളും വീട്ടുവിലാസങ്ങളും ഞങ്ങൾ സൂക്ഷിക്കുകയോ കാണിക്കുകയോ ചെയ്യുന്നില്ല; ഉറവിടത്തിലുണ്ടെങ്കിലും. അത്തരം വിവരം ഇവിടെ എവിടെയെങ്കിലും കണ്ടാൽ മുകളിലെ വിലാസത്തിൽ അറിയിക്കുക; ഉടൻ നീക്കും. നിയമപരമായ അറിയിപ്പുകളും അതേ വിലാസത്തിലേക്ക് അയക്കുക.",
                    "We do not keep or show private individuals' phone numbers or home addresses, even where a source prints them. If you find such information anywhere here, write to the address above and we will remove it promptly. Legal notices go to the same address.",
                )) }

                h2 #privacy { (lang.pick("സ്വകാര്യത", "Privacy")) }
                @match lang {
                    Lang::Ml => ul {
                        li { "അക്കൗണ്ടുകളില്ല, കുക്കികളില്ല, പരസ്യ ട്രാക്കറുകളില്ല." }
                        li { "ഓരോ തരം താളും ഓരോ ദിവസവും എത്ര തവണ തുറന്നു, വായനക്കാർ ഏത് സൈറ്റിൽ നിന്ന് വന്നു (പേര് മാത്രം), ഓരോ പദ്ധതിത്താളും എത്ര തവണ വായിച്ചു എന്നിവ മാത്രം എണ്ണുന്നു. ഒരു തിരച്ചിലിൽ ഒന്നും കിട്ടിയില്ലെങ്കിൽ ആ വാക്കുകളും എണ്ണവും സൂക്ഷിക്കുന്നു; ആര് തിരഞ്ഞു എന്നില്ല. ഇമെയിൽ വിലാസമോ ഫോൺ നമ്പറോ പോലെ തോന്നുന്നവ സൂക്ഷിക്കില്ല." }
                        li { "ഐ.പി വിലാസങ്ങൾ ഞങ്ങൾ സൂക്ഷിക്കുന്നില്ല. സൈറ്റ് പ്രവർത്തിക്കുന്നത് ക്ലൗഡ്ഫ്ലെയറിലാണ്; സേവനം നൽകുന്നതിനായി അവർ സാങ്കേതിക വിവരങ്ങൾ കൈകാര്യം ചെയ്യുന്നു." }
                        li { "ഇന്റർനെറ്റ് ഇല്ലാതെ വായിക്കാനായി നിങ്ങൾ തുറന്ന താളുകൾ നിങ്ങളുടെ ഉപകരണത്തിൽ തന്നെ സൂക്ഷിക്കുന്നു; അത് ഞങ്ങളിലേക്ക് എത്തുന്നില്ല." }
                    },
                    Lang::En => ul {
                        li { "No accounts, no cookies, no advertising trackers." }
                        li { "We count only how often each kind of page is opened each day, which site readers came from (its name only), and how often each project page is read. When a search finds nothing, we keep the words and a count, not who searched; text that looks like an email address or phone number is not kept." }
                        li { "We do not store IP addresses. The site runs on Cloudflare, which handles technical data in order to serve it." }
                        li { "Pages you open are saved on your own device so they can be read offline; that copy never reaches us." }
                    },
                }

                h2 #licences { (lang.pick("ലൈസൻസുകൾ", "Licences")) }
                p {
                    (lang.pick("ഡാറ്റ: CC BY 4.0 (", "Data: CC BY 4.0 ("))
                    a href=(data) { (lang.pick("വിശദമായി", "details")) }
                    (lang.pick("). സോഴ്സ് കോഡ്: AGPL-3.0, ", "). Source code: AGPL-3.0, "))
                    a href="https://github.com/TheSoloHacker47/kanakku" rel="noopener" { "GitHub" } "."
                }
            }
        }
    };
    layout(&page(lang, title, lead, "/about", origin, Nav::None, None), body)
}

fn view_kind_label(lang: Lang, kind: &str) -> String {
    match kind {
        "home" => lang.pick("മുൻതാൾ", "Front page"),
        "projects" => lang.pick("പദ്ധതി പട്ടിക", "Project list"),
        "project" => lang.pick("ഒരു പദ്ധതി", "A project"),
        "funding" => lang.pick("അനുവദിച്ചതും നൽകിയതും", "Approved and paid"),
        "funding_project" => lang.pick("ഒരു പദ്ധതിയുടെ തുകകൾ", "A project's payments"),
        "liability" => lang.pick("അറ്റകുറ്റപ്പണി ബാധ്യത", "Repair liability"),
        "contractors" | "contractor" => lang.pick("കരാറുകാർ", "Contractors"),
        "agencies" | "agency" => lang.pick("നിർവഹണ സ്ഥാപനങ്ങൾ", "Agencies"),
        "district" => lang.pick("ജില്ല", "A district"),
        "map" => lang.pick("ഭൂപടം", "Map"),
        "methodology" => lang.pick("രീതി", "Method"),
        "data" => lang.pick("ഡാറ്റ", "Data"),
        "status" => lang.pick("പ്രവർത്തന നില", "Status"),
        "about" => lang.pick("കണക്കിനെക്കുറിച്ച്", "About"),
        other => return other.to_string(),
    }
    .to_string()
}

/// Shown by the offline worker when a page that was never opened is asked for without a connection.
pub fn offline(lang: Lang, origin: &str) -> String {
    let title = lang.pick("ഇപ്പോൾ ഇന്റർനെറ്റ് ഇല്ല", "You are offline");
    let lead = lang.pick(
        "ഈ താൾ ഇതുവരെ ഈ ഉപകരണത്തിൽ തുറന്നിട്ടില്ല, അതിനാൽ ഇപ്പോൾ കാണിക്കാനാവില്ല. മുമ്പ് തുറന്ന താളുകൾ ഇന്റർനെറ്റ് ഇല്ലാതെയും വായിക്കാം.",
        "This page has not been opened on this device before, so it cannot be shown now. Pages you have already opened can still be read without a connection.",
    );
    let body = html! {
        (head(lang, title, lead))
        div.wrap {
            p.actions.sec {
                a.btn href=(format!("{}/", lang.prefix())) { (lang.t().site_name) }
                a.btn.ghost href=(format!("{}/projects", lang.prefix())) { (lang.t().nav_projects) }
            }
        }
    };
    layout(&page(lang, title, lead, "/offline", origin, Nav::None, None), body)
}

pub fn not_found(lang: Lang, origin: &str) -> String {
    let t = lang.t();
    let body = html! {
        (head(lang, t.not_found_title, t.not_found_body))
        div.wrap {
            p.actions.sec {
                a.btn href=(format!("{}/projects", lang.prefix())) { (t.back_to_list) }
                a.btn.ghost href=(format!("{}/", lang.prefix())) { (t.site_name) }
            }
        }
    };
    layout(&page(lang, t.not_found_title, t.not_found_body, "/", origin, Nav::None, None), body)
}

fn page<'a>(
    lang: Lang,
    title: &'a str,
    description: &'a str,
    path: &'a str,
    origin: &'a str,
    nav: Nav,
    last_checked: Option<&'a str>,
) -> Page<'a> {
    Page { lang, title, description, path, origin, nav, last_checked, head: Markup::default(), image: None }
}
