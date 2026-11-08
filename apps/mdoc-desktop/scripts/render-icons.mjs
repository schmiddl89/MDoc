import { Resvg } from '@resvg/resvg-js'
import fs from 'fs'
import path from 'path'
import { fileURLToPath } from 'url'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const root = path.resolve(__dirname, '..')
const iconSvgPath = path.resolve(root, '../../inputs/icon.svg')
const iconsDir = path.join(root, 'src-tauri/icons')

const svg = fs.readFileSync(iconSvgPath)
const resvg = new Resvg(svg, {
  fitTo: { mode: 'width', value: 1024 },
})
const png = resvg.render().asPng()
const out1024 = path.join(iconsDir, 'app-icon-1024.png')
fs.writeFileSync(out1024, png)
console.log('wrote', out1024, png.length)
