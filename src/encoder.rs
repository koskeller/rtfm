use anyhow::Result;
use markdown::ParseOptions;

// TODO .mdx
// - Remove <Image ...
// - Fix page heading block -------....
// - Figure out what wrong with tables
// - Single Headers  docs/02-app/01-building-your-application/01-routing/08-parallel-routes.mdx:5
// - Merge small chunks from one doc together
// - Do something with links
pub fn split_to_chunks(value: &str) -> Result<Vec<String>> {
    let mut chunks = Vec::new();
    let tree = markdown::to_mdast(value, &ParseOptions::gfm())
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
