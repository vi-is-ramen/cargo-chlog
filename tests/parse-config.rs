use cargo_chlog::cfg;
extern crate annotate_snippets as ans;

#[test]
fn test_parse_config() {
    let config = cfg::parse();

    assert_eq!(config.commits.ignore.as_ref().unwrap().len(), 1);

    if let Some(brief) = &config.commits.ignore.unwrap()[0].brief {
        if let cfg::Pat::Regex(pat) = brief {
            assert_eq!(pat, "[Bb]ump version.*?");
        }
    }
}
