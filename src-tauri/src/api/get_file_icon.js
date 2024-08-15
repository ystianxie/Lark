#!/usr/bin/env node
import { fileIconToBuffer } from 'file-icon';
import fs from 'fs';
const path = process.argv[2];

fileIconToBuffer(path)
    .then(icon => {
        console.log("ok");
        fs.writeFileSync("temp.txt", icon.toString('base64'), (err) => { });
        // fs.writeFileSync("temp.png", icon, (err) => { });
        process.exit(0);
    })
    .catch(err => {
        console.error(err);
        process.exit(1);
    });
