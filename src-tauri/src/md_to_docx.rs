use docx_rs::*;
use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};

/// Konversi teks markdown (#, ##, **bold**, *italic*, -, tabel) menjadi file .docx asli.
/// Ditulis manual (bukan cuma library polos) supaya heading, bold, list, dan tabel
/// benar-benar jadi elemen Word native — bukan teks datar.
pub fn markdown_to_docx_bytes(markdown: &str) -> Result<Vec<u8>, String> {
    let parser = Parser::new(markdown);

    let mut doc = Docx::new();

    // state builder
    let mut current_runs: Vec<Run> = Vec::new();
    let mut bold = false;
    let mut italic = false;
    let mut current_heading: Option<HeadingLevel> = None;
    let mut in_list_item = false;
    let mut list_ordered = false;
    let mut list_counter = 0;

    // table state
    let mut in_table = false;
    let mut table_rows: Vec<Vec<String>> = Vec::new();
    let mut current_row: Vec<String> = Vec::new();
    let mut current_cell_text = String::new();
    let mut table_header_done = false;

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Heading { level, .. } => {
                    current_heading = Some(level);
                }
                Tag::Strong => bold = true,
                Tag::Emphasis => italic = true,
                Tag::List(start) => {
                    list_ordered = start.is_some();
                    list_counter = start.unwrap_or(1) as i32;
                }
                Tag::Item => {
                    in_list_item = true;
                }
                Tag::Table(_) => {
                    in_table = true;
                    table_rows.clear();
                    table_header_done = false;
                }
                Tag::TableRow => {
                    current_row.clear();
                }
                Tag::TableCell => {
                    current_cell_text.clear();
                }
                _ => {}
            },
            Event::End(tag_end) => match tag_end {
                TagEnd::Heading(_) => {
                    doc = flush_heading(doc, &current_runs, current_heading);
                    current_runs.clear();
                    current_heading = None;
                }
                TagEnd::Strong => bold = false,
                TagEnd::Emphasis => italic = false,
                TagEnd::Paragraph => {
                    if !in_table {
                        doc = flush_paragraph(doc, &current_runs, in_list_item, list_ordered, list_counter);
                        current_runs.clear();
                    }
                }
                TagEnd::Item => {
                    in_list_item = false;
                    if list_ordered {
                        list_counter += 1;
                    }
                }
                TagEnd::TableCell => {
                    current_row.push(current_cell_text.clone());
                }
                TagEnd::TableRow => {
                    table_rows.push(current_row.clone());
                    if !table_header_done {
                        table_header_done = true;
                    }
                }
                TagEnd::Table => {
                    doc = flush_table(doc, &table_rows);
                    in_table = false;
                    table_rows.clear();
                }
                _ => {}
            },
            Event::Text(text) => {
                if in_table {
                    current_cell_text.push_str(&text);
                } else {
                    let mut run = Run::new().add_text(text.to_string());
                    if bold {
                        run = run.bold();
                    }
                    if italic {
                        run = run.italic();
                    }
                    current_runs.push(run);
                }
            }
            Event::Code(text) => {
                if in_table {
                    current_cell_text.push_str(&text);
                } else {
                    let run = Run::new().add_text(text.to_string()).fonts(
                        RunFonts::new().ascii("Consolas"),
                    );
                    current_runs.push(run);
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                if in_table {
                    current_cell_text.push(' ');
                } else {
                    current_runs.push(Run::new().add_break(BreakType::TextWrapping));
                }
            }
            Event::Rule => {
                doc = doc.add_paragraph(
                    Paragraph::new().add_run(Run::new().add_text("")).style("Normal"),
                );
            }
            _ => {}
        }
    }

    let mut buffer = Vec::new();
    let cursor = std::io::Cursor::new(&mut buffer);
    doc.build()
        .pack(cursor)
        .map_err(|e| format!("Gagal membangun docx: {:?}", e))?;

    Ok(buffer)
}

fn flush_heading(doc: Docx, runs: &[Run], level: Option<HeadingLevel>) -> Docx {
    let mut p = Paragraph::new();
    for r in runs.iter().cloned() {
        p = p.add_run(r);
    }
    let style_id = match level {
        Some(HeadingLevel::H1) => "Heading1",
        Some(HeadingLevel::H2) => "Heading2",
        Some(HeadingLevel::H3) => "Heading3",
        Some(HeadingLevel::H4) => "Heading4",
        _ => "Heading5",
    };
    p = p.style(style_id);
    doc.add_paragraph(p)
}

fn flush_paragraph(doc: Docx, runs: &[Run], is_list: bool, ordered: bool, counter: i32) -> Docx {
    if runs.is_empty() {
        return doc;
    }
    let mut p = Paragraph::new();
    let prefix = if is_list {
        if ordered {
            format!("{}. ", counter)
        } else {
            "•  ".to_string()
        }
    } else {
        String::new()
    };
    if !prefix.is_empty() {
        p = p.add_run(Run::new().add_text(prefix));
        p = p.indent(Some(360), None, None, None);
    }
    for r in runs.iter().cloned() {
        p = p.add_run(r);
    }
    doc.add_paragraph(p)
}

fn flush_table(mut doc: Docx, rows: &[Vec<String>]) -> Docx {
    if rows.is_empty() {
        return doc;
    }
    let col_count = rows.iter().map(|r| r.len()).max().unwrap_or(1).max(1);
    let table_width_dxa: i32 = 9000;
    let col_width = table_width_dxa / col_count as i32;
    let col_widths: Vec<i32> = vec![col_width; col_count];

    let mut table_rows: Vec<TableRow> = Vec::new();

    for (i, row) in rows.iter().enumerate() {
        let mut cells: Vec<TableCell> = Vec::new();
        for c in 0..col_count {
            let text = row.get(c).cloned().unwrap_or_default();
            let mut run = Run::new().add_text(text);
            if i == 0 {
                run = run.bold();
            }
            let mut cell = TableCell::new()
                .width(col_width as usize, WidthType::Dxa)
                .add_paragraph(Paragraph::new().add_run(run));
            if i == 0 {
                cell = cell.shading(
                    Shading::new()
                        .shd_type(ShdType::Clear)
                        .fill("D9E2F3"),
                );
            }
            cells.push(cell);
        }
        table_rows.push(TableRow::new(cells));
    }

    let table = Table::new(table_rows).set_grid(col_widths.iter().map(|w| *w as usize).collect());
    doc = doc.add_table(table);
    doc
}
