import {
  createTokenSwap,
  swap,
} from './swap-test';

export async function main() {
  // These test cases are designed to run sequentially and in the following order
  console.log('Run test: createTokenSwap');
  await createTokenSwap();
  //These test cases are designed to run sequentially and in the following order
  console.log('Run test: swap');
  await swap();
  
  console.log('Success\n');
}
  
main()
  .catch(err => {
    console.error(err);
    process.exit(-1);
  })
  .then(() => process.exit());