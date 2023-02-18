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
const VAULT_TOKEN_SOL_SEED = "vault-token-sol-seed";

// type ExType = IdlTypes<Wen3ex>["ExType"];

const ExTypeEnum = {
  TokenToSol: { tokenToSol: {} },
  SolToToken: { solToToken: {} },
};

describe("wen3ex token2sol", async () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());
  const provider = anchor.getProvider();
  const { connection } = provider;
  //   console.log(anchor.workspace);
  const program = anchor.workspace.Wen3Ex as Program<Wen3ex>;

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

  const rubyKP = Keypair.fromSecretKey(Buffer.from(JSON.parse(rubyText)));
  const creatorKP = Keypair.fromSecretKey(Buffer.from(JSON.parse(creatorText)));

  const takerKP = Keypair.fromSecretKey(Buffer.from(JSON.parse(takerText)));

  const takerRubyAmount = 1000;
  const creatorRubyAmount = 10;

  const mintAuthority = anchor.web3.Keypair.generate();
  const marketAccountKP = anchor.web3.Keypair.generate(); //

  it("wen3ex token2sol before", async () => {
    await airDrop(creatorKP.publicKey, 2);
    await airDrop(takerKP.publicKey, 2);
    let creatorSol = await printSolBalance(creatorKP.publicKey);
    console.log("before all", creatorSol);
    // 1. create mint
    await initMint(rubyKP, takerKP);

    let creatorSolBalance = await connection.getBalance(creatorKP.publicKey);
    console.log(0, { creatorSolBalance });

    const mintBalance = await connection.getBalance(rubyKP.publicKey);
    console.log({ mintBalance });

    // 2. tokenAccount , creator have sol, taker have ruby
    const takerRubyAta = await getATA(
      takerKP,
      rubyKP.publicKey,
      takerKP.publicKey
    );

    creatorSolBalance = await connection.getBalance(creatorKP.publicKey);
    console.log("00", { creatorSolBalance });

    // 3. mintTo.  taker got 1000 ruby
    await mintToken2Ata(
      takerKP,
      rubyKP.publicKey,
      takerRubyAta.address,
      takerRubyAmount
    );

    // const takerRubyAccount = await getAccount(connection, takerRubyAta.address);
    // console.log(Number(takerRubyAccount.amount));

    creatorSolBalance = await connection.getBalance(creatorKP.publicKey);
    console.log("000", { creatorSolBalance });

    const takerRubyAtaBalance = await connection.getBalance(
      takerRubyAta.address
    );
    console.log({ takerRubyAtaBalance });

    const takerRubyAccount = await getAccount(connection, takerRubyAta.address);
    console.log(Number(takerRubyAccount.amount));
    expect(Number(takerRubyAccount.amount)).to.eq(takerRubyAmount);
  });

  it("Create a market for buy ruby with sol", async () => {
    const [vaultPDA, vaultBump] = getVaultPDA();
    let creatorSolBalance = await connection.getBalance(creatorKP.publicKey);
    console.log(1, { creatorSolBalance });

    try {
      await createSol2TokenMarket();
    } catch (error) {
      console.log(error);
      throw error;
    }

    const marketAccount = await program.account.marketTsAccount.fetch(
      marketAccountKP.publicKey
    );

    expect(marketAccount.exType).to.haveOwnProperty("solToToken");
    expect(marketAccount.token.toBase58()).to.eq(rubyKP.publicKey.toBase58());
    expect(marketAccount.tokenAmount.toNumber()).to.eq(creatorRubyAmount);
    expect(marketAccount.solAmount.toNumber()).to.eq(LAMPORTS_PER_SOL);

    const vaultTokenAccountBalance = await connection.getBalance(vaultPDA);
    console.log({ vaultTokenAccountBalance });
    expect(vaultTokenAccountBalance).to.gt(LAMPORTS_PER_SOL);
    creatorSolBalance = await connection.getBalance(creatorKP.publicKey);
    console.log(2, { creatorSolBalance });
    expect(creatorSolBalance).to.lt(LAMPORTS_PER_SOL);
  });

  it("Close the market which buy ruby", async () => {
    let creatorRubyAta = await getATA(
      creatorKP,
      rubyKP.publicKey,
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
        creatorTokenAccount: creatorRubyAta.address,
        vaultTokenAccount: vaultPDA,
        vaultAuthority: vaultAuthorityPDA,
        marketAccount: marketAccountKP.publicKey,
        mint: rubyKP.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([creatorKP])
      .rpc();

    const vaultPDAClosed = await connection.getAccountInfo(vaultPDA);
    const marketAccountClosed = await connection.getAccountInfo(
      marketAccountKP.publicKey
    );
    expect(vaultPDAClosed).to.null;
    expect(marketAccountClosed).to.null;

    const creatorSolBalance = await connection.getBalance(creatorKP.publicKey);
    console.log(3, { creatorSolBalance });
  });

  it("Exchange the market sol 2 token", async () => {
    await createSol2TokenMarket();
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
    const takerRubyAta = await getATA(
      takerKP,
      rubyKP.publicKey,
      takerKP.publicKey
    );
    const takerRubyAccount = await getAccount(connection, takerRubyAta.address);
    console.log(Number(takerRubyAccount.amount));

    const creatorRubyAta = await getATA(
      creatorKP,
      rubyKP.publicKey,
      creatorKP.publicKey
    );
    try {
      await program.methods
        .marketTsExchange()
        .accounts({
          taker: takerKP.publicKey,
          takerTokenAccount: takerRubyAta.address,
          creator: creatorKP.publicKey,

          vaultTokenAccount: vaultPDA,
          vaultAuthority: vaultAuthorityPDA,
          marketAccount: marketAccountKP.publicKey,
          mint: rubyKP.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
        })

        .remainingAccounts([
          {
            pubkey: creatorRubyAta.address,
            isSigner: false,
            isWritable: true,
          },
        ])
        .signers([takerKP])
        .rpc();
    } catch (error) {
      console.log(error);
      throw error;
    }

    const takerSolBalance = await connection.getBalance(takerKP.publicKey);
    console.log({ takerSolBalance });
    const creatorSolBalance = await connection.getBalance(creatorKP.publicKey);
    console.log({ creatorSolBalance });
  });

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

  async function createSol2TokenMarket() {
    const [vaultPDA, vaultBump] = getVaultPDA();

    const creatorRubyAta = await getATA(
      creatorKP,
      rubyKP.publicKey,
      creatorKP.publicKey
    );

    console.log("marketAccountKP", marketAccountKP.publicKey.toBase58());

    await program.account.marketTsAccount.createInstruction(marketAccountKP);
    await program.methods
      .marketTsCreate(
        vaultBump,
        rubyKP.publicKey,
        new anchor.BN(creatorRubyAmount),
        new anchor.BN(LAMPORTS_PER_SOL),
        ExTypeEnum.SolToToken
      )
      .accounts({
        marketAccount: marketAccountKP.publicKey,
        vaultTokenAccount: vaultPDA,
        creatorTokenAccount: creatorRubyAta.address,
        mint: rubyKP.publicKey,
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
