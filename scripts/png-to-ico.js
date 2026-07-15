#!/usr/bin/env node
/**
 * Wraps a PNG file into a valid Windows .ico container (Vista+ PNG-in-ICO format).
 * This is the same format Tauri's own tooling produces when only PNG sources exist.
 *
 * Usage: node scripts/png-to-ico.js <input.png> <output.ico>
 */
const fs = require('fs');

const [,, input, output] = process.argv;
if (!input || !output) {
    console.error('Usage: node png-to-ico.js <input.png> <output.ico>');
    process.exit(1);
}

const png = fs.readFileSync(input);

// ICONDIR (6 bytes)
//  reserved : u16 = 0
//  type     : u16 = 1 (icon)
//  count    : u16 = 1
// ICONDIRENTRY (16 bytes)
//  width    : u8   (0 means 256)
//  height   : u8   (0 means 256)
//  colors   : u8   = 0
//  reserved : u8   = 0
//  planes   : u16  = 1
//  bitcount : u16  = 32
//  size     : u32  = png byte length
//  offset   : u32  = 22 (6 + 16)
const header = Buffer.alloc(6);
header.writeUInt16LE(0, 0); // reserved
header.writeUInt16LE(1, 2); // type = icon
header.writeUInt16LE(1, 4); // count = 1

// We do not decode the PNG dimensions here; using 0 for width/height
// is the standard sentinel value meaning "256" and is accepted by Windows
// for PNG-compressed icon entries (the actual size comes from the PNG).
const entry = Buffer.alloc(16);
entry.writeUInt8(0, 0);     // width  (0 => 256)
entry.writeUInt8(0, 1);     // height (0 => 256)
entry.writeUInt8(0, 2);     // color count (0 => >=256)
entry.writeUInt8(0, 3);     // reserved
entry.writeUInt16LE(1, 4);  // planes
entry.writeUInt16LE(32, 6); // bit count
entry.writeUInt32LE(png.length, 8);  // image size
entry.writeUInt32LE(22, 12);         // offset to image data

const ico = Buffer.concat([header, entry, png]);
fs.writeFileSync(output, ico);
console.log(`Wrote ${output} (${ico.length} bytes, PNG-in-ICO)`);
