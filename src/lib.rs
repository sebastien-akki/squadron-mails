mod mails;

use serde_json::{Value, json};
use squadron_extension_api::{Agent, Extension, host, squadron_extension};

struct Mails;

fn en() -> bool {
    host::state_get("lang").as_deref() == Some("en")
}

macro_rules! t {
    ($fr:literal, $en:literal $(, $a:expr)*) => {
        if en() { format!($en $(, $a)*) } else { format!($fr $(, $a)*) }
    };
}

const FALLBACK: &str = "http://localhost:8025";

fn option(id: &str) -> String {
    serde_json::from_str::<Value>(&host::options()).unwrap_or_default()[id].as_str().unwrap_or_default().trim().trim_end_matches('/').to_string()
}

fn run(program: &str, args: &[&str], dir: Option<&str>) -> Result<squadron_extension_api::Output, String> {
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    host::exec(program, &args, dir)
}

// The set address, else the project's Lando Mailpit or MailHog, else Mailpit's default port.
fn detect(folder: &str) -> String {
    let set = option("url");
    if !set.is_empty() {
        return set;
    }
    let found = run("lando", &["info", "--format", "json"], Some(folder)).ok().filter(|o| o.code == 0).and_then(|o| mails::lando_url(&o.stdout));
    let url = found.unwrap_or_else(|| FALLBACK.to_string());
    host::state_set(&format!("url:{folder}"), Some(&url));
    url
}

fn base(folder: &str) -> String {
    host::state_get(&format!("url:{folder}")).filter(|_| option("url").is_empty()).unwrap_or_else(|| detect(folder))
}

fn curl(method: &str, url: &str) -> Result<String, String> {
    let out = run("curl", &["-sk", "--max-time", "10", "-X", method, url], None)?;
    if out.code != 0 {
        return Err(out.stderr);
    }
    Ok(out.stdout)
}

// Tries the remembered address, then looks again once: the Lando app may have moved or stopped.
fn call(folder: &str, method: &str, path: &str) -> Result<(String, String), String> {
    let first = base(folder);
    if let Ok(body) = curl(method, &format!("{first}/{path}")) {
        return Ok((body, first));
    }
    let again = detect(folder);
    curl(method, &format!("{again}/{path}")).map(|b| (b, again.clone())).map_err(|_| {
        t!(
            "Mailpit ne répond pas à {}. Démarrez l’app Lando du projet, ou indiquez l’adresse de Mailpit dans les réglages de l’extension.",
            "Mailpit does not answer at {}. Start the project's Lando app, or set Mailpit's address in the extension's settings.",
            again
        )
    })
}

fn list(folder: &str, query: &str) -> Result<(Vec<mails::Mail>, u64, String), String> {
    let path = if query.trim().is_empty() { "api/v1/messages?limit=50".to_string() } else { format!("api/v1/search?limit=50&query={}", mails::encode(query.trim())) };
    let (body, url) = call(folder, "GET", &path)?;
    let (list, total) = mails::list(&body).ok_or_else(|| t!("{} ne répond pas comme Mailpit.", "{} does not answer like Mailpit.", url))?;
    Ok((list, total, url))
}

fn read(folder: &str, id: &str) -> Result<mails::Detail, String> {
    let (body, url) = call(folder, "GET", &format!("api/v1/message/{}", mails::encode(id)))?;
    mails::detail(&body).ok_or_else(|| t!("{} n’a pas trouvé ce mail.", "{} did not find this mail.", url))
}

fn post(message: Value) {
    host::panel_post("main", &message.to_string());
}

fn listing(folder: &str, query: &str) -> Value {
    match list(folder, query) {
        Ok((mails, total, url)) => json!({ "type": "list", "mails": mails, "total": total, "url": url }),
        Err(e) => json!({ "type": "error", "message": e }),
    }
}

fn answer(kind: &str, agent: &Agent, m: &Value) -> Result<Value, String> {
    let text = |k: &str| m[k].as_str().unwrap_or_default().to_string();
    Ok(match kind {
        "ready" => {
            if let Some(lang) = m["lang"].as_str() {
                host::state_set("lang", Some(if lang.starts_with("fr") { "fr" } else { "en" }));
            }
            listing(&agent.project, "")
        }
        "list" => listing(&agent.project, &text("query")),
        "open" => json!({ "type": "detail", "detail": read(&agent.project, &text("id"))? }),
        "clear" => {
            call(&agent.project, "DELETE", "api/v1/messages")?;
            listing(&agent.project, "")
        }
        _ => return Err(format!("Unknown request {kind}.")),
    })
}

impl Extension for Mails {
    fn on_event(name: String, agent: Option<Agent>, data: String) {
        if name != "panel.message" {
            return;
        }
        let data: Value = serde_json::from_str(&data).unwrap_or_default();
        let m = &data["message"];
        let Some(agent) = agent else { return };
        let kind = m["type"].as_str().unwrap_or_default();
        let mut reply = answer(kind, &agent, m).unwrap_or_else(|e| json!({ "type": "error", "message": e }));
        reply["to"] = m["to"].clone();
        post(reply);
    }

    fn call_tool(name: String, input: String, agent: Option<Agent>) -> Result<String, String> {
        let agent = agent.ok_or("Call this tool from an agent started by Squadron.")?;
        let input: Value = serde_json::from_str(&input).unwrap_or_default();
        match name.as_str() {
            "list" => {
                let (mails, total, url) = list(&agent.project, input["query"].as_str().unwrap_or_default())?;
                if mails.is_empty() {
                    return Ok(format!("No mail in Mailpit ({url}) for this search."));
                }
                let lines: Vec<String> = mails.iter().map(|m| format!("{} · {} · {} → {} · {}", m.id, m.created, m.from, m.to, m.subject)).collect();
                Ok(format!("{total} mail(s) in Mailpit ({url}), newest first:\n{}", lines.join("\n")))
            }
            "read" => {
                let id = input["id"].as_str().map(str::trim).filter(|s| !s.is_empty()).ok_or("Give the id of the mail, as mails_list gives it.")?;
                let d = read(&agent.project, id)?;
                let links = if d.links.is_empty() { String::new() } else { format!("\n\nLinks:\n{}", d.links.join("\n")) };
                Ok(format!("From: {}\nTo: {}\nDate: {}\nSubject: {}\n\n{}{links}", d.mail.from, d.mail.to, d.mail.created, d.mail.subject, d.text.trim()))
            }
            _ => Err(format!("Unknown tool {name}.")),
        }
    }
}

squadron_extension!(Mails);
