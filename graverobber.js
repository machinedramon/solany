const fs = require("fs");
const path = require("path");
const ROOT_DIR = __dirname;
const IGNORE_FILE = path.join(ROOT_DIR, "graverobber.js");

function shouldIgnore(filePath) {
  const fileName = path.basename(filePath);
  const ignoredExtensions = [
    ".bin",
    ".svg",
    ".jpg",
    ".jpeg",
    ".png",
    ".gif",
    ".mp3",
    ".mp4",
    ".wav",
    ".ico",
    ".sqlite",
    ".lock",
    ".md",
  ];
  return (
    filePath === IGNORE_FILE ||
    ignoredExtensions.some((ext) => fileName.endsWith(ext)) ||
    filePath.includes(path.sep + ".git" + path.sep) ||
    filePath.endsWith(".git") ||
    filePath.includes(path.sep + "public" + path.sep) ||
    filePath.endsWith("public") ||
    filePath.includes(path.sep + "target" + path.sep) ||
    filePath.endsWith("target") ||
    filePath.includes(path.sep + "assets" + path.sep) ||
    filePath.endsWith("assets") ||
    filePath.includes(path.sep + ".next" + path.sep) ||
    filePath.endsWith(".next") ||
    filePath.includes(path.sep + "node_modules" + path.sep) ||
    filePath.endsWith("node_modules") ||
    fileName === "yarn.lock"
  );
}

function scanDir(currentDir) {
  let results = {};
  fs.readdirSync(currentDir).forEach((file) => {
    const fullPath = path.join(currentDir, file);
    if (shouldIgnore(fullPath)) return;
    const stats = fs.statSync(fullPath);
    if (stats.isDirectory()) {
      results[file] = scanDir(fullPath);
    } else {
      results[file] = {
        path: fullPath,
        content: fs.readFileSync(fullPath, "utf8").replace(/\s+/g, ""),
      };
    }
  });
  return results;
}

function generateTree(dir, indent = "") {
  let tree = "";
  fs.readdirSync(dir).forEach((file) => {
    const fullPath = path.join(dir, file);
    if (shouldIgnore(fullPath)) return;
    const stats = fs.statSync(fullPath);
    if (stats.isDirectory()) {
      tree += `${indent}📂${file}\n`;
      tree += generateTree(fullPath, indent + "  ");
    } else {
      tree += `${indent}📄${file}\n`;
    }
  });
  return tree;
}

function saveReport(tree, report) {
  const filePath = path.join(ROOT_DIR, "report.txt");
  const content = `FileTree:\n${tree}\n\nDetails:\n${JSON.stringify(report)}`;
  fs.writeFileSync(filePath, content, "utf8");
  console.log(
    "🖤 GRAVEROBBER: A coleta foi realizada. O relatório sombrio está pronto. 🖤"
  );
}

function main() {
  console.log(
    "🪦 GRAVEROBBER: O ritual de varredura começou. Não há escapatória... ⚰️"
  );
  const tree = generateTree(ROOT_DIR);
  const report = scanDir(ROOT_DIR);
  console.log(
    "🔍 GRAVEROBBER: A varredura terminou. Preparando o grimório dos arquivos... 📜"
  );
  saveReport(tree, report);
  console.log(
    "📜 GRAVEROBBER: O grimório está completo. Os segredos dos arquivos foram revelados. Use-os com sabedoria, ou sofra as consequências... 😈"
  );
}

main();
