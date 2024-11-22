use fancy_regex::Regex;
use std::{error::Error, iter};

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

    let lines = source.split(line_sep).map(|line| line.to_owned());

    // ====================================
    // mach 3 puts probe data in #2002, but UCNC puts it in #5063
    let lines = lines.map(|line| line.replace("#2002", "#5063"));

    // ====================================
    // remove "pause program" lines
    let lines = lines.filter(|line| *line != "M0 (PAUSE PROGRAM)");

    // ====================================
    // add sleep after M3 ("switch spindle on")
    let sleep_duration_ms = 8000;
    let lines = lines.flat_map(|line| {
        if line.starts_with("M3 ") {
            vec![
                line,
                format!("G4 P{sleep_duration_ms} (m3tu: added sleep after M3)"),
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
        let matched = line_with_subexpr_pat.captures(&line);

        type Res = Box<dyn Iterator<Item = Result<String, String>>>;

        // note: we added some lines in the previous pass, so we can't just take the index
        let line_no = -1; // TODO
        if line.starts_with(&temp_var_used_prefix) {
            let msg = format!("Temp variable '${temp_var}' is assigned to on line ${line_no}");
            return Box::new(iter::once(Err::<String, String>(msg))) as Res;
        }

        let res: Res = match matched {
            Ok(Some(caps)) => {
                let axis = caps.get(0).unwrap().as_str();
                let expr = caps.get(1).unwrap().as_str();

                if expr.contains("[") {
                    let msg = format!(
                        "Nested brackets are not supported\n\n  {line}\n\n(line {line_no})"
                    );
                    // return vec![msg].into_iter().map(|msg| Err::<String, String>(msg));
                    return Box::new(iter::once(Err::<String, String>(msg))) as Res;
                }

                // TODO: technically we're only replacing the first expr, there could be multiple
                let trimmed = line.trim();
                let res = [
                    format!("(m3tu: \"{trimmed}\")"),
                    format!("#{temp_var} = [{expr}] (m3tu: extracted subexpression for {axis})"),
                    subexpr_arg_pat
                        .replace(&line, format!("#{temp_var}"))
                        .to_string(),
                ]
                .into_iter();

                Box::new(res.map(|line| Ok::<String, String>(line))) as Res
            }
            _ => Box::new(iter::once(Ok::<String, String>(line))) as Res,
        };
        res
    });

    let collected = {
        let result: Result<Vec<String>, _> = lines.collect();
        result?
    };

    for line in collected.iter() {
        println!("{}", &line);
    }

    Ok(())
}
