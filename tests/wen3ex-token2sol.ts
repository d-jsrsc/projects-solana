import fs from "node:fs";
import path from "node:path";

import * as anchor from "@project-serum/anchor";
import { IdlTypes, Program } from "@project-serum/anchor";
import { Keypair, LAMPORTS_PER_SOL, PublicKey } from "@solana/web3.js";
import {
  getAccount,
  TOKEN_PROGRAM_ID,
  createMint,
  mintTo,
  getOrCreateAssociatedTokenAccount,
} from "@solana/spl-token";
import { assert, expect } from "chai";

import { Wen3ex, IDL } from "../target/types/wen3ex";

const VAULT_AUTHORITY_SEED = "vault-authority-seed";
const VAULT_TOKEN_SOL_SEED = "vault-token-2-sol-seed";

describe("wen3ex token2sol", async () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());
  const provider = anchor.getProvider();
  const { connection } = provider;
  //   console.log(anchor.workspace);
  const program = anchor.workspace.Wen3Ex as Program<Wen3ex>;

  const goldText = await fs.readFileSync(
    path.join(__dirname, "./private/gold.json"),
    "utf-8"
  );

  const creatorText = await fs.readFileSync(
    path.join(__dirname, "./private/creator.json"),
    "utf-8"
  );

  const takerText = await fs.readFileSync(
    path.join(__dirname, "./private/taker.json"),
    "utf-8"
  );

  const goldKP = Keypair.fromSecretKey(Buffer.from(JSON.parse(goldText)));
  const creatorKP = Keypair.fromSecretKey(Buffer.from(JSON.parse(creatorText)));

  const takerKP = Keypair.fromSecretKey(Buffer.from(JSON.parse(takerText)));

  const creatorAmount = 2000;
  const creatorDepositAmount = 20;

  const mintAuthority = anchor.web3.Keypair.generate();
  const marketAccountKP = anchor.web3.Keypair.generate(); //

  it("wen3ex token2sol before", async () => {
    await airDrop(creatorKP.publicKey, 2);
    await airDrop(takerKP.publicKey, 2);
    let creatorSol = await printSolBalance(creatorKP.publicKey);
    console.log("before all", creatorSol);
    // 1. create mint
    await initMint(goldKP, creatorKP);

    const mintBalance = await connection.getBalance(goldKP.publicKey);
    console.log({ mintBalance });

    creatorSol = await printSolBalance(creatorKP.publicKey);
    console.log("before after mint", creatorSol);
    // 2. tokenAccount , creator have gold, taker have ruby
    const creatorGoldAta = await getATA(
      creatorKP,
      goldKP.publicKey,
      creatorKP.publicKey
    );

    // 3. mintTo. creator got 2000 gold, taker got 1000 ruby
    await mintToken2Ata(
      creatorKP,
      goldKP.publicKey,
      creatorGoldAta.address,
      creatorAmount
    );

    const creatorGoldAtaBalance = await connection.getBalance(
      creatorGoldAta.address
    );
    console.log({ creatorGoldAtaBalance });

    creatorSol = await printSolBalance(creatorKP.publicKey);
    console.log("before after mintToken2Ata", creatorSol);
    const creatorGoldAccount = await getAccount(
      connection,
      creatorGoldAta.address
    );
    expect(Number(creatorGoldAccount.amount)).to.eq(creatorAmount);
  });

  it("wen3ex token2sol is initialized!", async () => {
    // Add your test here.
    const tx = await program.methods.initialize().rpc();
    console.log("Your transaction signature", tx);
  });

  it("Create marketAccount token 2 sol", async () => {
    let creatorSol = await printSolBalance(creatorKP.publicKey);
    console.log(1, { creatorSol }); // 1996499120
    const creatorGoldAta = await getATA(
      creatorKP,
      goldKP.publicKey,
      creatorKP.publicKey
    );
    creatorSol = await printSolBalance(creatorKP.publicKey);
    console.log(2, { creatorSol }); // 1996499120
    let creatorGoldTokenAccount = await getAccount(
      connection,
      creatorGoldAta.address
    );
    creatorSol = await printSolBalance(creatorKP.publicKey);
    console.log(3, { creatorSol }); // 1996499120

    expect(Number(creatorGoldTokenAccount.amount)).to.eq(creatorAmount);

    const [vaultPDA, _vaultBump] = getVaultPDA();
    const [vaultAuthorityPDA] = getVaultAuthorityPDA();

    await createToken2SolMarket();

    creatorSol = await printSolBalance(creatorKP.publicKey);
    console.log("after create market", { creatorSol }); // 1994459840

    let vaultTokenAccount = await getAccount(connection, vaultPDA);

    let marketAccount = await program.account.marketTsAccount.fetch(
      marketAccountKP.publicKey
    );

    console.log(Number(marketAccount.createTime));
    expect(marketAccount.tokenAmount.toNumber()).to.eq(creatorDepositAmount);

    assert.ok(vaultTokenAccount.owner.equals(vaultAuthorityPDA));
    assert.ok(marketAccount.creator.equals(creatorKP.publicKey));
    assert.ok(marketAccount.token.equals(goldKP.publicKey));

    const creatorGoldAccount = await getAccount(
      connection,
      creatorGoldAta.address
    );
    // creatorRubyAccount = await getAccount(connection, creatorRubyAta.address);

    expect(Number(creatorGoldAccount.amount)).to.eq(
      creatorAmount - creatorDepositAmount
    );

    const allTsMarkets = await program.account.marketTsAccount.all();
    expect(allTsMarkets.length).to.eq(1);

    const filter = [
      {
        memcmp: {
          offset: 8 + 4,
          bytes: creatorKP.publicKey.toBase58(),
        },
      },
    ];

    const userTsMarkets = await program.account.marketTsAccount.all(filter);
    expect(userTsMarkets.length).to.eq(1);

    creatorSol = await printSolBalance(creatorKP.publicKey);
    console.log(4, { creatorSol }); // 1994459840
  });

  it("Close marketAccount token 2 sol without exchange", async () => {
    let creatorGoldAta = await getATA(
      creatorKP,
      goldKP.publicKey,
      creatorKP.publicKey
    );
    const [vaultPDA, _vaultAccountBump] = PublicKey.findProgramAddressSync(
      [Buffer.from(VAULT_TOKEN_SOL_SEED), marketAccountKP.publicKey.toBuffer()],
      program.programId
    );
    const [vaultAuthorityPDA, _vaultAuthorityBump] =
      PublicKey.findProgramAddressSync(
        [
          Buffer.from(VAULT_AUTHORITY_SEED),
          marketAccountKP.publicKey.toBuffer(),
        ],
        program.programId
      );

    const vaultBalance = await connection.getBalance(vaultPDA);
    console.log({ vaultBalance });
    const marketAccountBalance = await connection.getBalance(
      marketAccountKP.publicKey
    );
    console.log({ marketAccountBalance });
    await program.methods
      .marketTsCancel()
      .accounts({
        creator: creatorKP.publicKey,
        creatorTokenAccount: creatorGoldAta.address,
        vaultTokenAccount: vaultPDA,
        vaultAuthority: vaultAuthorityPDA,
        marketAccount: marketAccountKP.publicKey,
        mint: goldKP.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([creatorKP])
      .rpc();

    creatorGoldAta = await getAccount(connection, creatorGoldAta.address);
    assert.ok(creatorGoldAta.owner.equals(creatorKP.publicKey));
    assert.ok(Number(creatorGoldAta.amount) == creatorAmount);
    console.log("creatorGoldAta", creatorGoldAta.amount);
    const creatorGoldAtaBalance = await connection.getBalance(
      creatorGoldAta.address
    );
    const creatorSol = await printSolBalance(creatorKP.publicKey);
    const goldMintBalance = await printSolBalance(goldKP.publicKey);
    console.log("close", {
      creatorSol,
      creatorGoldAtaBalance,
      goldMintBalance,
    }); // 1998280880
    // PDAs should be closed
    const vaultPDAClosed = await connection.getAccountInfo(vaultPDA);
    const marketAccountClosed = await connection.getAccountInfo(
      marketAccountKP.publicKey
    );
    expect(vaultPDAClosed).to.null;
    expect(marketAccountClosed).to.null;
  });

  // before all 2000000000
  // before after mint 1998538400
  // before after mintToken2Ata 1996499120
  // Before exchange { creatorSol: 1996499120, takerSol: 2000000000 }
  // After create market exchange { creatorSol: 1994459840, takerSol: 2000000000 }
  // ---- { creatorSol: 1994459840, takerSol: 1997960720 }
  // After exchange { creatorBalance: 2998280880, takerBalance: 997960720 }
  it("Exchange token 2 sol", async () => {
    let creatorSol = await printSolBalance(creatorKP.publicKey);
    let takerSol = await printSolBalance(takerKP.publicKey);
    console.log("Before exchange", { creatorSol, takerSol }); // 1998280880, 2000000000

    await createToken2SolMarket();

    creatorSol = await printSolBalance(creatorKP.publicKey);
    takerSol = await printSolBalance(takerKP.publicKey);
    console.log("After create market exchange", { creatorSol, takerSol }); // 1996241600, 2000000000

    let takerGoldAta = await getATA(
      takerKP,
      goldKP.publicKey,
      takerKP.publicKey
    );

    let creatorGoldAta = await getATA(
      creatorKP,
      goldKP.publicKey,
      creatorKP.publicKey
    );

    creatorSol = await printSolBalance(creatorKP.publicKey);
    takerSol = await printSolBalance(takerKP.publicKey);
    console.log("----", { creatorSol, takerSol }); // 1996241600, 1997960720

    expect(Number(creatorGoldAta.amount)).to.eq(
      creatorAmount - creatorDepositAmount
    );

    expect(Number(takerGoldAta.amount)).to.eq(0);

    const [vaultPDA, _vaultBump] = getVaultPDA();
    const [vaultAuthorityPDA] = getVaultAuthorityPDA();
    const data = await connection.getAccountInfo(vaultAuthorityPDA);
    console.log({ data });

    await program.methods
      .marketTsExchange()
      .accounts({
        taker: takerKP.publicKey,
        takerTokenAccount: takerGoldAta.address,

        creator: creatorKP.publicKey,
        marketAccount: marketAccountKP.publicKey,
        vaultTokenAccount: vaultPDA,
        mint: goldKP.publicKey,
        vaultAuthority: vaultAuthorityPDA,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([takerKP])
      .rpc();

    takerGoldAta = await getATA(takerKP, goldKP.publicKey, takerKP.publicKey);
    creatorGoldAta = await getATA(
      creatorKP,
      goldKP.publicKey,
      creatorKP.publicKey
    );
    expect(Number(creatorGoldAta.amount)).to.eq(
      creatorAmount - creatorDepositAmount
    );
    expect(Number(takerGoldAta.amount)).to.eq(creatorDepositAmount);

    const creatorBalance = await printSolBalance(creatorKP.publicKey);
    const takerBalance = await printSolBalance(takerKP.publicKey);
    console.log("After exchange", { creatorBalance, takerBalance }); // 3000062640, 997960720

    // expect(creatorBalance).to.lt(3 * LAMPORTS_PER_SOL);
    // PDAs should be closed
    const vaultPDAClosed = await connection.getAccountInfo(vaultPDA);
    const marketAccountClosed = await connection.getAccountInfo(
      marketAccountKP.publicKey
    );
    expect(vaultPDAClosed).to.null;
    expect(marketAccountClosed).to.null;
  });

  // creator got 2000 gold

  async function printSolBalance(key: PublicKey) {
    const solBalance = await connection.getBalance(key);
    return solBalance;
  }

  function getVaultPDA() {
    return PublicKey.findProgramAddressSync(
      [Buffer.from(VAULT_TOKEN_SOL_SEED), marketAccountKP.publicKey.toBuffer()],
      program.programId
    );
  }

  function getVaultAuthorityPDA() {
    return PublicKey.findProgramAddressSync(
      [Buffer.from(VAULT_AUTHORITY_SEED), marketAccountKP.publicKey.toBuffer()],
      program.programId
    );
  }

  async function createToken2SolMarket() {
    const [vaultPDA, vaultBump] = getVaultPDA();

    const creatorGoldAta = await getATA(
      creatorKP,
      goldKP.publicKey,
      creatorKP.publicKey
    );

    await program.methods
      .marketTsCreate(
        new anchor.BN(creatorDepositAmount),
        new anchor.BN(LAMPORTS_PER_SOL)
      )
      .accounts({
        marketAccount: marketAccountKP.publicKey,
        vaultTokenAccount: vaultPDA,
        creatorTokenAccount: creatorGoldAta.address,
        mint: goldKP.publicKey,
        creator: creatorKP.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .preInstructions([
        await program.account.marketTsAccount.createInstruction(
          marketAccountKP
        ),
      ])
      .signers([marketAccountKP, creatorKP])
      .rpc();
  }

  async function mintToken2Ata(
    payer: Keypair,
    mint: PublicKey,
    ata: PublicKey,
    amount: number
  ) {
    await mintTo(connection, payer, mint, ata, mintAuthority, amount);
  }

  async function getATA(payer: Keypair, mint: PublicKey, owner: PublicKey) {
    const ata = await getOrCreateAssociatedTokenAccount(
      connection,
      payer,
      mint,
      owner
    );
    return ata;
  }

  async function initMint(kp: Keypair, payer: Keypair) {
    await createMint(connection, payer, mintAuthority.publicKey, null, 0, kp);
  }

  async function airDrop(pubkey: PublicKey, num: number) {
    const airdropSignature = await connection.requestAirdrop(
      pubkey,
      num * LAMPORTS_PER_SOL
    );
    await confirmSignature(airdropSignature);
  }

  async function confirmSignature(signature: string) {
    const latestBlockHash = await connection.getLatestBlockhash();
    await connection.confirmTransaction({
      blockhash: latestBlockHash.blockhash,
      lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
      signature: signature,
    });
  }
});
