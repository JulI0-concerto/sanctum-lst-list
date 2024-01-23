import { writeFile } from "fs/promises";
import { SanctumLstList } from "./loader";
import type { LST } from "./types";

async function writeDataToFile(data: LST[]) {
  const dataStr = `import { type LST } from './types';\n\nexport const LstList: LST[] = ${JSON.stringify(
    data,
    null,
    2,
  )};`;
  await writeFile("./lstData.ts", dataStr);
}

const lstList = await SanctumLstList.load();

await writeDataToFile(lstList);
