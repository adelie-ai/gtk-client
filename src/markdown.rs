use pulldown_cmark::{Options, Parser, html};

/// Convert markdown text to HTML.
pub fn markdown_to_html(input: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(input, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    html_output
}

/// Render a full set of chat messages into an HTML document body.
pub fn render_messages_html(
    messages: &[(String, String)],
    streaming_buffer: Option<&str>,
) -> String {
    let mut html = String::new();

    for (role, content) in messages {
        let (class, label) = match role.as_str() {
            "user" => ("message user-message", "You"),
            "assistant" => ("message assistant-message", "Adele"),
            _ => ("message", ""),
        };

        let content_html = markdown_to_html(content);
        html.push_str(&format!(
            r#"<div class="{class}"><div class="label">{label}</div><div class="content">{content_html}</div></div>"#
        ));
    }

    if let Some(buffer) = streaming_buffer {
        if !buffer.is_empty() {
            let content_html = markdown_to_html(buffer);
            html.push_str(&format!(
                r#"<div class="message assistant-message streaming"><div class="label">Adele</div><div class="content">{content_html}<span class="cursor">▌</span></div></div>"#
            ));
        }
    }

    html
}

/// Full HTML page template with embedded CSS.
pub fn html_template() -> &'static str {
    r##"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<style>
* { margin: 0; padding: 0; box-sizing: border-box; }

body {
    background: #1a1d2e;
    color: #e0e0e0;
    font-family: system-ui, -apple-system, sans-serif;
    font-size: 14px;
    line-height: 1.6;
    padding: 16px;
}

#messages {
    display: flex;
    flex-direction: column;
    gap: 16px;
}

.message {
    border-radius: 8px;
    padding: 12px 16px;
}

.user-message {
    background: rgba(255, 189, 89, 0.08);
    border-left: 3px solid #ffbd59;
}

.user-message .label {
    color: #ffbd59;
    font-weight: 600;
    margin-bottom: 4px;
}

.assistant-message {
    background: rgba(92, 206, 154, 0.08);
    border-left: 3px solid #5cce9a;
}

.assistant-message .label {
    color: #5cce9a;
    font-weight: 600;
    margin-bottom: 4px;
}

.assistant-message.streaming {
    border-left-color: #84dac1;
}

.assistant-message.streaming .label {
    color: #84dac1;
}

.content p { margin: 0.5em 0; }
.content p:first-child { margin-top: 0; }
.content p:last-child { margin-bottom: 0; }

.content pre {
    background: #232740;
    border-radius: 6px;
    padding: 12px;
    overflow-x: auto;
    margin: 0.5em 0;
}

.content code {
    font-family: 'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace;
    font-size: 13px;
}

.content :not(pre) > code {
    background: #232740;
    padding: 2px 6px;
    border-radius: 3px;
}

.content ul, .content ol {
    padding-left: 1.5em;
    margin: 0.5em 0;
}

.content table {
    border-collapse: collapse;
    margin: 0.5em 0;
}

.content th, .content td {
    border: 1px solid #3a3f5c;
    padding: 6px 12px;
}

.content th {
    background: #232740;
}

.content a {
    color: #7aa3ff;
    text-decoration: none;
}

.content a:hover {
    text-decoration: underline;
}

.cursor {
    color: #84dac1;
    animation: blink 1s step-end infinite;
}

@keyframes blink {
    50% { opacity: 0; }
}
</style>
</head>
<body>
<div id="messages"></div>
<script>
function updateMessages(html) {
    document.getElementById('messages').innerHTML = html;
    scrollToBottom();
}

function appendChunk(text) {
    // Find streaming message or create one
    let streaming = document.querySelector('.streaming .content');
    if (!streaming) {
        let div = document.createElement('div');
        div.className = 'message assistant-message streaming';
        div.innerHTML = '<div class="label">Adele</div><div class="content"></div>';
        document.getElementById('messages').appendChild(div);
        streaming = div.querySelector('.content');
    }
    // Append raw text (for streaming, we accumulate and re-render on complete)
    streaming.textContent += text;
    scrollToBottom();
}

function scrollToBottom() {
    window.scrollTo(0, document.body.scrollHeight);
}
</script>
</body>
</html>"##
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_markdown_conversion() {
        let html = markdown_to_html("**bold** and *italic*");
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("<em>italic</em>"));
    }

    #[test]
    fn code_block_conversion() {
        let md = "```rust\nfn main() {}\n```";
        let html = markdown_to_html(md);
        assert!(html.contains("<code"));
        assert!(html.contains("fn main()"));
    }

    #[test]
    fn render_messages_produces_html() {
        let messages = vec![
            ("user".to_string(), "Hello".to_string()),
            ("assistant".to_string(), "Hi there!".to_string()),
        ];
        let html = render_messages_html(&messages, None);
        assert!(html.contains("user-message"));
        assert!(html.contains("assistant-message"));
        assert!(html.contains("Hello"));
        assert!(html.contains("Hi there!"));
    }

    #[test]
    fn render_with_streaming_buffer() {
        let messages = vec![];
        let html = render_messages_html(&messages, Some("Partial..."));
        assert!(html.contains("streaming"));
        assert!(html.contains("Partial..."));
        assert!(html.contains("cursor"));
    }

    #[test]
    fn html_template_is_valid() {
        let template = html_template();
        assert!(template.contains("<!DOCTYPE html>"));
        assert!(template.contains("updateMessages"));
        assert!(template.contains("#messages"));
    }
}
