use anyhow::{Result, anyhow};
use quick_xml::escape::unescape;
use quick_xml::events::Event;
use quick_xml::{Reader, XmlVersion};

#[derive(Debug, Clone, PartialEq)]
pub struct VideoDanmaku {
    pub time: f64,
    pub text: String,
    pub color: u32,
}

pub fn parse_xml(xml: &str) -> Result<Vec<VideoDanmaku>> {
    let mut reader = Reader::from_str(xml);
    let mut danmaku = Vec::new();
    loop {
        match reader.read_event()? {
            Event::Start(event) if event.name().as_ref() == "d" => {
                let metadata = event
                    .attributes()
                    .flatten()
                    .find(|attribute| attribute.key.as_ref() == "p")
                    .ok_or_else(|| anyhow!("danmaku entry is missing p metadata"))?
                    .normalized_value(XmlVersion::Implicit1_0)?;
                let mut fields = metadata.split(',');
                let time = fields.next().and_then(|value| value.parse().ok());
                let _mode = fields.next();
                let _size = fields.next();
                let color = fields.next().and_then(|value| value.parse().ok());
                let raw_text = reader.read_text(event.name())?;
                let text = unescape(&raw_text)?.into_owned();
                if let (Some(time), Some(color)) = (time, color)
                    && time >= 0.0
                    && !text.is_empty()
                {
                    danmaku.push(VideoDanmaku { time, text, color });
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    danmaku.sort_by(|left, right| left.time.total_cmp(&right.time));
    Ok(danmaku)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_timed_xml_danmaku() {
        let parsed = parse_xml(r#"<i><d p="1.5,1,25,16711680,0,0,0,0">A&amp;B</d></i>"#).unwrap();
        assert_eq!(
            parsed,
            vec![VideoDanmaku {
                time: 1.5,
                text: "A&B".into(),
                color: 16_711_680
            }]
        );
    }

    #[tokio::test]
    #[ignore = "requires network access"]
    async fn current_public_danmaku_xml_parses() {
        let parsed = crate::api::client::ApiClient::new()
            .get_video_danmaku(39884818572)
            .await
            .unwrap();
        assert!(!parsed.is_empty());
    }
}
