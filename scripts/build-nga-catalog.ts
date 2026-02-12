#!/usr/bin/env bun
/**
 * Downloads NGA open data CSVs and builds a compact catalog JSON
 * of paintings with IIIF image UUIDs for embedding into the app.
 *
 * Usage: bun scripts/build-nga-catalog.ts
 * Output: src-tauri/resources/nga_catalog.json
 */

const IMAGES_URL =
  "https://raw.githubusercontent.com/NationalGalleryOfArt/opendata/main/data/published_images.csv";
const OBJECTS_URL =
  "https://raw.githubusercontent.com/NationalGalleryOfArt/opendata/main/data/objects.csv";
const OUTPUT = "src-tauri/resources/nga_catalog.json";

interface NgaEntry {
  uuid: string;
  title: string;
  artist: string;
  date: string;
  medium: string;
}

function parseCSV(text: string): Record<string, string>[] {
  const lines = text.split("\n");
  const headers = parseCSVLine(lines[0]);
  const rows: Record<string, string>[] = [];

  let i = 1;
  while (i < lines.length) {
    // Handle multi-line quoted fields
    let line = lines[i];
    while (countQuotes(line) % 2 !== 0 && i + 1 < lines.length) {
      i++;
      line += "\n" + lines[i];
    }
    i++;

    if (!line.trim()) continue;
    const values = parseCSVLine(line);
    const row: Record<string, string> = {};
    for (let j = 0; j < headers.length; j++) {
      row[headers[j]] = values[j] ?? "";
    }
    rows.push(row);
  }
  return rows;
}

function countQuotes(s: string): number {
  let count = 0;
  for (const c of s) if (c === '"') count++;
  return count;
}

function parseCSVLine(line: string): string[] {
  const result: string[] = [];
  let current = "";
  let inQuotes = false;

  for (let i = 0; i < line.length; i++) {
    const c = line[i];
    if (inQuotes) {
      if (c === '"') {
        if (i + 1 < line.length && line[i + 1] === '"') {
          current += '"';
          i++;
        } else {
          inQuotes = false;
        }
      } else {
        current += c;
      }
    } else {
      if (c === '"') {
        inQuotes = true;
      } else if (c === ",") {
        result.push(current.trim());
        current = "";
      } else {
        current += c;
      }
    }
  }
  result.push(current.trim());
  return result;
}

async function main() {
  console.log("Downloading NGA published_images.csv...");
  const imagesResp = await fetch(IMAGES_URL);
  const imagesText = await imagesResp.text();
  console.log(`  Downloaded ${(imagesText.length / 1024 / 1024).toFixed(1)}MB`);

  console.log("Downloading NGA objects.csv...");
  const objectsResp = await fetch(OBJECTS_URL);
  const objectsText = await objectsResp.text();
  console.log(`  Downloaded ${(objectsText.length / 1024 / 1024).toFixed(1)}MB`);

  console.log("Parsing images CSV...");
  const images = parseCSV(imagesText);
  console.log(`  ${images.length} image records`);

  console.log("Parsing objects CSV...");
  const objects = parseCSV(objectsText);
  console.log(`  ${objects.length} object records`);

  // Build object lookup by objectid
  const objectMap = new Map<string, Record<string, string>>();
  for (const obj of objects) {
    objectMap.set(obj.objectid, obj);
  }

  // Filter: primary view images that link to painting objects
  console.log("Joining and filtering to paintings...");
  const catalog: NgaEntry[] = [];
  const seenObjects = new Set<string>();

  for (const img of images) {
    if (img.viewtype !== "primary") continue;
    if (!img.uuid) continue;

    const objectId = img.depictstmsobjectid;
    if (!objectId || seenObjects.has(objectId)) continue;

    const obj = objectMap.get(objectId);
    if (!obj) continue;

    // Filter to paintings
    const classification = obj.classification?.toLowerCase() ?? "";
    if (!classification.includes("painting")) continue;

    // Skip if no title
    const title = obj.title?.trim();
    if (!title) continue;

    seenObjects.add(objectId);
    catalog.push({
      uuid: img.uuid,
      title,
      artist: obj.attribution || obj.attributioninverted || "Unknown Artist",
      date: obj.displaydate || "",
      medium: obj.medium || "",
    });
  }

  console.log(`  ${catalog.length} paintings with images`);

  // Write compact JSON
  const json = JSON.stringify(catalog);
  await Bun.write(OUTPUT, json);
  console.log(
    `Written to ${OUTPUT} (${(json.length / 1024).toFixed(0)}KB, ${catalog.length} entries)`
  );
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
