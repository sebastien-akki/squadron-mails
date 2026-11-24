use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct Mail {
    pub id: String,
    pub from: String,
    pub to: String,
    pub subject: String,
    pub created: String,
    pub snippet: String,
    pub read: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct Detail {
    pub mail: Mail,
    pub text: String,
    pub html: String,
    pub links: Vec<String>,
}

fn s(v: &Value) -> String {
    v.as_str().map(String::from).unwrap_or_default()
}

fn address(v: &Value) -> String {
    let (name, addr) = (s(&v["Name"]), s(&v["Address"]));
    if name.is_empty() { addr } else { format!("{name} <{addr}>") }
}

fn mail(v: &Value) -> Mail {
    Mail {
        id: s(&v["ID"]),
        from: address(&v["From"]),
        to: v["To"].as_array().into_iter().flatten().map(address).collect::<Vec<_>>().join(", "),
        subject: s(&v["Subject"]),
        created: [s(&v["Created"]), s(&v["Date"])].into_iter().find(|d| !d.is_empty()).unwrap_or_default(),
        snippet: s(&v["Snippet"]),
        read: v["Read"] == true,
    }
}

pub fn list(body: &str) -> Option<(Vec<Mail>, u64)> {
    let v: Value = serde_json::from_str(body).ok()?;
    let mails = v["messages"].as_array()?.iter().map(mail).collect();
    Some((mails, v["messages_count"].as_u64().or_else(|| v["total"].as_u64()).unwrap_or(0)))
}

pub fn links(html: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for part in html.split("href=").skip(1) {
        let quote = part.chars().next().unwrap_or(' ');
        let link: String = if quote == '"' || quote == '\'' { part[1..].chars().take_while(|c| *c != quote).collect() } else { part.chars().take_while(|c| !c.is_whitespace() && *c != '>').collect() };
        let link = link.replace("&amp;", "&");
        if link.starts_with("http") && !out.contains(&link) {
            out.push(link);
        }
    }
    out
}

pub fn detail(body: &str) -> Option<Detail> {
    let v: Value = serde_json::from_str(body).ok()?;
    let html = s(&v["HTML"]);
    Some(Detail { mail: mail(&v), text: s(&v["Text"]), links: links(&html), html })
}

// Lando's Mailpit or MailHog service, over plain HTTP so that curl needs no certificate.
pub fn lando_url(info: &str) -> Option<String> {
    let v: Value = serde_json::from_str(info.get(info.find('[')?..)?).ok()?;
    let service = v.as_array()?.iter().find(|s| ["service", "type"].iter().any(|k| s[k].as_str().is_some_and(|x| x.contains("mailpit") || x.contains("mailhog"))))?;
    let urls: Vec<String> = service["urls"].as_array()?.iter().filter_map(Value::as_str).map(String::from).collect();
    urls.iter().find(|u| u.starts_with("http://")).or_else(|| urls.first()).map(|u| u.trim_end_matches('/').to_string())
}

pub fn encode(text: &str) -> String {
    text.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIST: &str = r#"{"total":1,"messages_count":1,"messages":[{"ID":"5amb","From":{"Name":"Boutique","Address":"shop@exemple.test"},"To":[{"Name":"Client","Address":"client@exemple.test"}],"Subject":"Confirmation de votre commande #1042","Created":"2026-11-24T19:45:49.148Z","Snippet":"Merci pour votre commande #1042.","Read":false}]}"#;

    #[test]
    fn reads_the_list() {
        let (mails, total) = list(LIST).unwrap();
        assert_eq!(total, 1);
        assert_eq!((mails[0].from.as_str(), mails[0].to.as_str()), ("Boutique <shop@exemple.test>", "Client <client@exemple.test>"));
        assert!(list("404 page not found").is_none());
    }

    #[test]
    fn reads_a_message_and_its_links() {
        let body = r#"{"ID":"5amb","From":{"Name":"","Address":"shop@exemple.test"},"To":[],"Subject":"S","Date":"2026-11-24T19:45:49Z","Text":"Merci","HTML":"<a href=\"https://shop.test/orders/1?a=1&amp;b=2\">Voir</a> <a href='https://shop.test/x'>x</a> <a href=\"mailto:a@b\">m</a> <a href=\"https://shop.test/x\">again</a>"}"#;
        let d = detail(body).unwrap();
        assert_eq!(d.mail.from, "shop@exemple.test");
        assert_eq!(d.links, ["https://shop.test/orders/1?a=1&b=2", "https://shop.test/x"]);
    }

    #[test]
    fn finds_mailpit_in_lando_info() {
        let info = r#"Some warning
[{"service":"appserver_nginx","type":"symfony-nginx","urls":["http://maximo.lndo.site:8000/"]},{"service":"mailpit","type":"compose","urls":["https://mailpit-maximo.lndo.site/","http://mailpit-maximo.lndo.site:8000/"]}]"#;
        assert_eq!(lando_url(info).as_deref(), Some("http://mailpit-maximo.lndo.site:8000"));
        assert_eq!(lando_url(r#"[{"service":"appserver","type":"php","urls":[]}]"#), None);
        assert_eq!(encode("subject:commande #1"), "subject%3Acommande%20%231");
    }
}
