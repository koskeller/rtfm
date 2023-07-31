use anyhow::Result;
use markdown::ParseOptions;
use regex::Regex;

pub fn split_by_headings(value: &str) -> Result<Vec<String>> {
    let mut chunks = Vec::new();
    let tree = markdown::to_mdast(value, &ParseOptions::default())
        .map_err(|err| anyhow::anyhow!("Failed to build markdown tree {}", err))?;
    let mut prev_offset = 0;
    let root = tree.children().unwrap();
    for node in root {
        match node {
            markdown::mdast::Node::Heading(heading) => {
                if heading.depth > 3 {
                    continue;
                }
                if let Some(pos) = &heading.position {
                    let chunk = &value[prev_offset..pos.start.offset];
                    if chunk.len() > 8 {
                        chunks.push(chunk.to_owned());
                    }
                    prev_offset = pos.start.offset;
                }
            }
            _ => {}
        }
    }
    Ok(chunks)
}

#[derive(Debug)]
pub struct Head {
    pub subcategory: String,
    pub layout: String,
    pub title: String,
    pub desc: String,
}

pub fn extract_head_values(input: &str) -> Head {
    let subcategory_re = Regex::new(r#"subcategory: \"(.*?)\""#).unwrap();
    let layout_re = Regex::new(r#"layout: \"(.*?)\""#).unwrap();
    let title_re = Regex::new(r#"page_title: \"(.*?)\""#).unwrap();
    let desc_re = Regex::new(r#"description: \|-\s*(.*)"#).unwrap();

    let subcategory = subcategory_re
        .captures(input)
        .and_then(|cap| cap.get(1))
        .map_or("", |m| m.as_str());
    let layout = layout_re
        .captures(input)
        .and_then(|cap| cap.get(1))
        .map_or("", |m| m.as_str());
    let title = title_re
        .captures(input)
        .and_then(|cap| cap.get(1))
        .map_or("", |m| m.as_str());
    let desc = desc_re
        .captures(input)
        .and_then(|cap| cap.get(1))
        .map_or("", |m| m.as_str());

    Head {
        subcategory: subcategory.to_string(),
        layout: layout.to_string(),
        title: title.to_string(),
        desc: desc.to_string(),
    }
}

pub fn extract_head(input: &str) -> Option<String> {
    let parts: Vec<&str> = input.split("---").collect();
    if parts.len() < 3 || parts.len() > 3 {
        return None;
    }
    Some(parts[1].to_string())
}

pub fn remove_head(input: String) -> String {
    let parts: Vec<&str> = input.split("---").collect();
    if parts.len() < 3 || parts.len() > 3 {
        return input;
    }
    parts[2].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_head() {
        let input = r#"---subcategory: "ACM"---Other content"#;
        let head = extract_head(input);
        assert_eq!(head, Some("subcategory: \"ACM\"".to_string()));
    }

    #[test]
    fn test_extract_head_with_no_delimiter() {
        let input = "Some text";
        let head = extract_head(input);
        assert!(head.is_none());
    }

    #[test]
    fn test_extract_head_with_one_delimiter() {
        let input = r#"subcategory: "ACM"---Other content"#;
        let head = extract_head(input);
        assert!(head.is_none());
    }

    #[test]
    fn test_extract_head_values() {
        let input = r#"subcategory: "ACM (Certificate Manager)"
layout: "aws"
page_title: "AWS: aws_acm_certificate"
description: |-
  Get information on a Amazon Certificate Manager (ACM) Certificate"#;

        let head = extract_head_values(input);

        assert_eq!(head.subcategory, "ACM (Certificate Manager)");
        assert_eq!(head.layout, "aws");
        assert_eq!(head.title, "AWS: aws_acm_certificate");
        assert_eq!(
            head.desc,
            "Get information on a Amazon Certificate Manager (ACM) Certificate"
        );
    }

    #[test]
    fn test_extract_head_values_with_missing_values() {
        let input = "";

        let head = extract_head_values(input);

        assert_eq!(head.subcategory, "");
        assert_eq!(head.layout, "");
        assert_eq!(head.title, "");
        assert_eq!(head.desc, "");
    }
}
