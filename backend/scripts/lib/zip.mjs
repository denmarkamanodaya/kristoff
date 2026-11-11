// Deterministic single-entry zip writer (Lambda bundles). Fixed timestamp 2000-01-01 00:00 and mode
// 0644, so the same input bytes always give the same zip bytes (and the same Terraform hash).
import { crc32, deflateRawSync } from 'node:zlib';

const DOS_DATE = ((2000 - 1980) << 9) | (1 << 5) | 1;

/** A zip holding exactly one deflated file, with fixed metadata. */
export function zipOne(name, data) {
  const nameBytes = Buffer.from(name, 'ascii');
  const body = deflateRawSync(data, { level: 9 });
  const crc = crc32(data) >>> 0;
  const header = (size, sig, extra) => {
    const b = Buffer.alloc(size);
    b.writeUInt32LE(sig, 0);
    extra(b);
    return b;
  };
  const common = (b, at) => {
    b.writeUInt16LE(20, at); // version needed
    b.writeUInt16LE(0, at + 2); // flags
    b.writeUInt16LE(8, at + 4); // deflate
    b.writeUInt16LE(0, at + 6); // time 00:00
    b.writeUInt16LE(DOS_DATE, at + 8);
    b.writeUInt32LE(crc, at + 10);
    b.writeUInt32LE(body.length, at + 14);
    b.writeUInt32LE(data.length, at + 18);
    b.writeUInt16LE(nameBytes.length, at + 22);
  };
  const local = header(30, 0x04034b50, (b) => common(b, 4));
  const central = header(46, 0x02014b50, (b) => {
    b.writeUInt16LE((3 << 8) | 20, 4); // made by unix
    common(b, 6);
    b.writeUInt32LE((0o100644 << 16) >>> 0, 38); // -rw-r--r--
  });
  const offset = local.length + nameBytes.length + body.length;
  const end = header(22, 0x06054b50, (b) => {
    b.writeUInt16LE(1, 8);
    b.writeUInt16LE(1, 10);
    b.writeUInt32LE(central.length + nameBytes.length, 12);
    b.writeUInt32LE(offset, 16);
  });
  return Buffer.concat([local, nameBytes, body, central, nameBytes, end]);
}
