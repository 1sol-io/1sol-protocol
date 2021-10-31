import {
  loadAllAmmInfos,
} from './swap-test';

export async function main() {

  await loadAllAmmInfos();

  console.log('Success\n');
}
  
main()
  .catch(err => {
    console.error(err);
    process.exit(-1);
  })
  .then(() => process.exit());