import * as fs from "fs";

const [inputFile] = Bun.argv.slice(2);
if (!inputFile) {
  console.error("Expected filename as argument");
  process.exit(1);
}

let source = fs.readFileSync(inputFile, "utf-8");

const LINE_SEP = source.includes("\r\n") ? "\r\n" : "\n";

// ====================================
// mach 3 puts probe data in #2002, but UCNC puts it in #5063
source = source.replaceAll("#2002", "#5063");

let lines = source.split(LINE_SEP);

// ====================================
// remove "pause program" lines
lines = lines.filter((line) => line !== "M0 (PAUSE PROGRAM)");

// ====================================
// add sleep after M3 ("switch spindle on")
SLEEP_DURATION_MS = 8000;
lines = lines.flatMap((line) => {
  if (line.startsWith("M3 ")) {
    return [line, `G4 P${SLEEP_DURATION_MS} (m3tu: added sleep after M3)`];
  }
  return [line];
});

// ====================================
// outline [subexpressions] from instructions like:
// G1 X 6.9210 Y 2.1120 Z[-0.100+#100]
const TEMP_VAR = "103";
lines = lines.flatMap((line, lineIndex) => {
  const match = line.match(/^(?<!#).*?(X|Y|Z)\[([^\]]+)\]/);
  if (line.startsWith(`#${TEMP_VAR} = `)) {
    throw new Error(
      `Temp variable '${TEMP_VAR}' is assigned to on line ${lineIndex + 1}`
    );
  }
  if (!match) {
    return [line];
  }
  const [, axis, expr] = match;
  if (expr.includes("[")) {
    throw new Error([
      "Nested brackets are not supported",
      "",
      "  " + line,
      "",
      `(line ${lineIndex + 1})`,
    ]);
  }

  return [
    `(m3tu: "${line.trim()}")`,
    `#${TEMP_VAR} = [${expr}] (m3tu: extracted subexpression for ${axis})`,
    line.replace(/(?<=X|Y|Z)\[[^\]]+\]/, `#${TEMP_VAR}`),
  ];
});

// ====================================
const newSource = lines.join(LINE_SEP);
console.log(newSource);
