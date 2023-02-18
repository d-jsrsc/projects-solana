import fs from "node:fs";
import path from "node:path";

import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { Keypair, LAMPORTS_PER_SOL, PublicKey } from "@solana/web3.js";
import {
  getAccount,
  TOKEN_PROGRAM_ID,
  createMint,
  mintTo,
  getOrCreateAssociatedTokenAccount,
} from "@solana/spl-token";
import { assert, expect } from "chai";

import { Wen3ex } from "../target/types/wen3ex";

const VAULT_AUTHORITY_SEED = "vault-authority-seed";
const VAULT_TOKEN_2_TOKEN_SEED = "vault-token-2-token-seed";

describe("wen3ex token2token", async () => {
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
  const rubyText = await fs.readFileSync(
    path.join(__dirname, "./private/ruby.json"),
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
  const rubyKP = Keypair.fromSecretKey(Buffer.from(JSON.parse(rubyText)));
  const creatorKP = Keypair.fromSecretKey(Buffer.from(JSON.parse(creatorText)));
  const takerKP = Keypair.fromSecretKey(Buffer.from(JSON.parse(takerText)));

  // let creatorGoldAta: Account | null = null;
  // let creatorRubyAta: Account | null = null;
  // let takerGoldAta: Account | null = null;
  // let takerRubyAta: Account | null = null;

  const takerAmount = 1000;
  const takerDepositAmount = 10;
  const creatorAmount = 2000;
  const creatorDepositAmount = 20;

  const mintAuthority = anchor.web3.Keypair.generate();
  const marketAccountKP = anchor.web3.Keypair.generate(); //

  it.skip("Is initialized!", async () => {
    // Add your test here.
    const tx = await program.methods.initialize().rpc();
    console.log("Your transaction signature", tx);
  });

  it.skip("wen3ex token2token before", async () => {
    await airDrop(creatorKP.publicKey, 2);
    await airDrop(takerKP.publicKey, 2);

    // 1. create mint
    await initMint(goldKP, creatorKP);
    await initMint(rubyKP, takerKP);

    // 2. tokenAccount , creator have gold, taker have ruby
    const creatorGoldAta = await getATA(
      creatorKP,
      goldKP.publicKey,
      creatorKP.publicKey
    );
    const takerRubyAta = await getATA(
      takerKP,
      rubyKP.publicKey,
      takerKP.publicKey
    );

    // 3. mintTo. creator got 2000 gold, taker got 1000 ruby
    await mintToken2Ata(
      creatorKP,
      goldKP.publicKey,
      creatorGoldAta.address,
      creatorAmount
    );
    await mintToken2Ata(
      takerKP,
      rubyKP.publicKey,
      takerRubyAta.address,
      takerAmount
    );

    const creatorRubyAta = await getATA(
      creatorKP,
      rubyKP.publicKey,
      creatorKP.publicKey
    );
    const creatorRubyAccount = await getAccount(
      connection,
      creatorRubyAta.address
    );
    expect(Number(creatorRubyAccount.amount)).to.eq(0);

    const creatorGoldAccount = await getAccount(
      connection,
      creatorGoldAta.address
    );
    const takerRubyAccount = await getAccount(connection, takerRubyAta.address);

    expect(Number(creatorGoldAccount.amount)).to.eq(creatorAmount);
    expect(Number(takerRubyAccount.amount)).to.eq(takerAmount);
  });

  it.skip("Create marketAccount token 2 token", async () => {
    const creatorRubyAta = await getATA(
      creatorKP,
      rubyKP.publicKey,
      creatorKP.publicKey
    );
    const creatorGoldAta = await getATA(
      creatorKP,
      goldKP.publicKey,
      creatorKP.publicKey
    );
    let creatorGoldAccount = await getAccount(
      connection,
      creatorGoldAta.address
    );
    let creatorRubyAccount = await getAccount(
      connection,
      creatorRubyAta.address
    );

    expect(Number(creatorGoldAccount.amount)).to.eq(creatorAmount);
    expect(Number(creatorRubyAccount.amount)).to.eq(0);

    const [vaultPDA, _vaultBump] = getVaultPDA();
    const [vaultAuthorityPDA] = getVaultAuthorityPDA();

    await createT2tMarket();

    let vaultTokenAccount = await getAccount(connection, vaultPDA);

    let marketAccount = await program.account.marketTtAccount.fetch(
      marketAccountKP.publicKey
    );

    // console.log(Number(marketAccount.createTime));
    expect(marketAccount.depositAmount.toNumber()).to.eq(creatorDepositAmount);
    expect(marketAccount.receiveAmount.toNumber()).to.eq(takerDepositAmount);

    assert.ok(vaultTokenAccount.owner.equals(vaultAuthorityPDA));
    assert.ok(marketAccount.creator.equals(creatorKP.publicKey));
    assert.ok(marketAccount.depositToken.equals(goldKP.publicKey));
    assert.ok(marketAccount.receiveToken.equals(rubyKP.publicKey));

    creatorGoldAccount = await getAccount(connection, creatorGoldAta.address);
    creatorRubyAccount = await getAccount(connection, creatorRubyAta.address);

    expect(Number(creatorGoldAccount.amount)).to.eq(
      creatorAmount - creatorDepositAmount
    );
    expect(Number(creatorRubyAccount.amount)).to.eq(0);

    const allMarketTT = await program.account.marketTtAccount.all();
    expect(allMarketTT.length).to.eq(1);

    const filter = [
      {
        memcmp: {
          offset: 8,
          bytes: creatorKP.publicKey.toBase58(),
        },
      },
    ];

    const userTTMarkets = await program.account.marketTtAccount.all(filter);
    expect(userTTMarkets.length).to.eq(1);
  });

  it.skip("Close marketAccount token 2 token without exchange", async () => {
    let creatorGoldAta = await getATA(
      creatorKP,
      goldKP.publicKey,
      creatorKP.publicKey
    );
    const [vaultPDA, _vaultAccountBump] = PublicKey.findProgramAddressSync(
      [
        Buffer.from(VAULT_TOKEN_2_TOKEN_SEED),
        marketAccountKP.publicKey.toBuffer(),
      ],
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
    await program.methods
      .marketTtCancel()
      .accounts({
        creator: creatorKP.publicKey,
        depositTokenAccount: creatorGoldAta.address,
        vaultTokenAccount: vaultPDA,
        vaultAuthority: vaultAuthorityPDA,
        marketAccount: marketAccountKP.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([creatorKP])
      .rpc();

    creatorGoldAta = await getAccount(connection, creatorGoldAta.address);
    assert.ok(creatorGoldAta.owner.equals(creatorKP.publicKey));
    assert.ok(Number(creatorGoldAta.amount) == creatorAmount);
  });

  it.skip("Exchange token 2 token", async () => {
    await createT2tMarket();

    let takerRubyAta = await getATA(
      takerKP,
      rubyKP.publicKey,
      takerKP.publicKey
    );
    let takerGoldAta = await getATA(
      takerKP,
      goldKP.publicKey,
      takerKP.publicKey
    );
    let creatorRubyAta = await getATA(
      creatorKP,
      rubyKP.publicKey,
      creatorKP.publicKey
    );
    let creatorGoldAta = await getATA(
      creatorKP,
      goldKP.publicKey,
      creatorKP.publicKey
    );
    expect(Number(creatorRubyAta.amount)).to.eq(0);
    expect(Number(creatorGoldAta.amount)).to.eq(
      creatorAmount - creatorDepositAmount
    );

    expect(Number(takerGoldAta.amount)).to.eq(0);
    expect(Number(takerRubyAta.amount)).to.eq(takerAmount);

    const [vaultPDA, _vaultBump] = getVaultPDA();
    const [vaultAuthorityPDA] = getVaultAuthorityPDA();
    await program.methods
      .marketTtExchange()
      .accounts({
        taker: takerKP.publicKey,
        takerDepositTokenAccount: takerRubyAta.address,
        takerReceiveTokenAccount: takerGoldAta.address,
        creatorDepositTokenAccount: creatorGoldAta.address,
        creatorReceiveTokenAccount: creatorRubyAta.address,

        creator: creatorKP.publicKey,
        marketAccount: marketAccountKP.publicKey,
        vaultTokenAccount: vaultPDA,
        vaultAuthority: vaultAuthorityPDA,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([takerKP])
      .rpc();

    takerRubyAta = await getATA(takerKP, rubyKP.publicKey, takerKP.publicKey);
    takerGoldAta = await getATA(takerKP, goldKP.publicKey, takerKP.publicKey);
    creatorRubyAta = await getATA(
      creatorKP,
      rubyKP.publicKey,
      creatorKP.publicKey
    );
    creatorGoldAta = await getATA(
      creatorKP,
      goldKP.publicKey,
      creatorKP.publicKey
    );

    expect(Number(creatorRubyAta.amount)).to.eq(takerDepositAmount);
    expect(Number(creatorGoldAta.amount)).to.eq(
      creatorAmount - creatorDepositAmount
    );

    expect(Number(takerGoldAta.amount)).to.eq(creatorDepositAmount);
    expect(Number(takerRubyAta.amount)).to.eq(takerAmount - takerDepositAmount);

    // PDAs should be closed
    const vaultPDAClosed = await connection.getAccountInfo(vaultPDA);
    const marketAccountClosed = await connection.getAccountInfo(
      marketAccountKP.publicKey
    );
    expect(vaultPDAClosed).to.null;
    expect(marketAccountClosed).to.null;
  });

  function getVaultPDA() {
    return PublicKey.findProgramAddressSync(
      [
        Buffer.from(VAULT_TOKEN_2_TOKEN_SEED),
        marketAccountKP.publicKey.toBuffer(),
      ],
      program.programId
    );
  }

  function getVaultAuthorityPDA() {
    return PublicKey.findProgramAddressSync(
      [Buffer.from(VAULT_AUTHORITY_SEED), marketAccountKP.publicKey.toBuffer()],
      program.programId
    );
  }

  async function createT2tMarket() {
    const [vaultPDA, vaultBump] = getVaultPDA();
    console.log("vaultPDA", vaultPDA.toBase58(), vaultBump);
    const creatorGoldAta = await getATA(
      creatorKP,
      goldKP.publicKey,
      creatorKP.publicKey
    );
    const creatorRubyAta = await getATA(
      creatorKP,
      rubyKP.publicKey,
      creatorKP.publicKey
    );
    try {
      await program.methods
        .marketTtCreate(
          new anchor.BN(20),
          new anchor.BN(10),
          goldKP.publicKey,
          rubyKP.publicKey
        )
        .accounts({
          creator: creatorKP.publicKey,
          marketAccount: marketAccountKP.publicKey,
          vaultTokenAccount: vaultPDA,
          mint: goldKP.publicKey,
          depositTokenAccount: creatorGoldAta.address,
          receiveTokenAccount: creatorRubyAta.address,
          systemProgram: anchor.web3.SystemProgram.programId,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .preInstructions([
          await program.account.marketTtAccount.createInstruction(
            marketAccountKP
          ),
        ])
        .signers([marketAccountKP, creatorKP])
        .rpc();
    } catch (error) {
      console.error(error);
      throw error;
    }
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
