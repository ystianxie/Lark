import { fileIconToBuffer, fileIconToFile } from 'file-icon';
// 获取文件图标
const path = "/Users/starsxu/Documents/office/阳光厨房_嘉兴_2024-06-13.xlsx";
await fileIconToFile(path, { destination: 'safari-icon.png' });

fileIconToBuffer(path)
    .then(icon => {
        console.log(icon.toString('base64'));
        process.exit(0);
    })
    .catch(err => {
        console.error(err);
        process.exit(1);
    });
