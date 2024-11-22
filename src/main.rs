use fancy_regex::Regex;
use std::{error::Error, iter};

struct SourceLine {
    text: String,
    loc: i64,
}

impl SourceLine {
    fn synthetic(text: String) -> SourceLine {
        SourceLine { text, loc: -1 }
    }

    fn is_synthetic(&self) -> bool {
        self.loc == -1
    }

    fn format_lineno(&self) -> String {
        if self.is_synthetic() {
            "<generated code>".to_string()
        } else {
            format!("{}", self.loc)
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    let Some(input_file) = args.get(1) else {
        eprintln!("Usage: mach3-to-ucnc <file>");
        std::process::exit(1);
    };

    let source = std::fs::read_to_string(input_file)?;

    let line_sep = if source.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };

    let lines = source
        .split(line_sep)
        .enumerate()
        .map(|(i, line)| SourceLine {
            text: line.to_owned(),
            loc: i as i64 + 1,
        });

    // ====================================
    // mach 3 puts probe data in #2002, but UCNC puts it in #5063
    let lines = lines.map(|line| SourceLine {
        text: line.text.replace("#2002", "#5063"),
        ..line
    });

    // ====================================
    // remove "pause program" lines
    let lines = lines.filter(|line| line.text != "M0 (PAUSE PROGRAM)");

    // ====================================
    // add sleep after M3 ("switch spindle on")
    let sleep_duration_ms = 8000;
    let lines = lines.flat_map(|line| {
        if line.text.starts_with("M3 ") {
            vec![
                line,
                SourceLine::synthetic(format!(
                    "G4 P{sleep_duration_ms} (m3tu: added sleep after M3)"
                )),
            ]
            .into_iter()
        } else {
            vec![line].into_iter()
        }
    });

    // ====================================
    // outline [subexpressions] from instructions like:
    // G1 X 6.9210 Y 2.1120 Z[-0.100+#100]
    let temp_var = "103";
    let temp_var_used_prefix = format!("#{temp_var} = ");

    let line_with_subexpr_pat = Regex::new(r"^(?<!#).*?(X|Y|Z)\[([^\]]+)\]").unwrap();
    let subexpr_arg_pat = Regex::new(r"(?<=X|Y|Z)\[[^\]]+\]").unwrap();

    let lines = lines.flat_map(|line| {
        let matched = line_with_subexpr_pat.captures(&line.text);

        type ResIter = Box<dyn Iterator<Item = Result<SourceLine, String>>>;

        if line.text.starts_with(&temp_var_used_prefix) {
            let line_no = line.format_lineno();
            let msg = format!("Temp variable '{temp_var}' is assigned to on line {line_no}");
            return Box::new(iter::once(Err::<SourceLine, String>(msg))) as ResIter;
        }

        let res: ResIter = match matched {
            Ok(Some(caps)) => {
                let axis = caps.get(0).unwrap().as_str();
                let expr = caps.get(1).unwrap().as_str();

                if expr.contains("[") {
                    let text = &line.text;
                    let line_no = line.format_lineno();
                    let msg = format!(
                        "Nested brackets are not supported\n\n  {text}\n\n(line {line_no})"
                    );
                    // return vec![msg].into_iter().map(|msg| Err::<SourceLine, String>(msg));
                    return Box::new(iter::once(Err::<SourceLine, String>(msg))) as ResIter;
                }

                // TODO: technically we're only replacing the first expr, there could be multiple
                let trimmed = line.text.trim();
                let res = [
                    SourceLine::synthetic(format!("(m3tu: \"{trimmed}\")")),
                    SourceLine::synthetic(format!(
                        "#{temp_var} = [{expr}] (m3tu: extracted subexpression for {axis})"
                    )),
                    SourceLine {
                        text: subexpr_arg_pat
                            .replace(&line.text, format!("#{temp_var}"))
                            .to_string(),
                        ..line
                    },
                ]
                .into_iter();

                Box::new(res.map(|line| Ok::<SourceLine, String>(line))) as ResIter
            }
            _ => Box::new(iter::once(Ok::<SourceLine, String>(line))) as ResIter,
        };
        res
    });

    let collected = {
        let result: Result<Vec<SourceLine>, _> = lines.collect();
        result?
    };

    for line in collected.iter() {
        println!("{}", line.text);
    }

    Ok(())
}
