import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import {
  TOKEN_PROGRAM_ID,
  getAssociatedTokenAddressSync,
  createAssociatedTokenAccountInstruction,
  createInitializeMint2Instruction,
  createMintToInstruction,
  MINT_SIZE,
  getMinimumBalanceForRentExemptMint,
} from "@solana/spl-token";
import {
  PublicKey,
  Keypair,
  SystemProgram,
  SYSVAR_RENT_PUBKEY,
  LAMPORTS_PER_SOL,
} from "@solana/web3.js";
import { expect } from "chai";
import { SolanaStablecoinAmmDex } from "../target/types/solana_stablecoin_amm_dex";

describe("solana-stablecoin-amm-dex", () => {
  // Configure the client
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.SolanaStablecoinAmmDex as Program<SolanaStablecoinAmmDex>;

  // Test accounts
  const admin = provider.wallet;
  const user = Keypair.generate();
  const protocolFeeRecipient = Keypair.generate();

  // Token mints
  let tokenAMint: Keypair;
  let tokenBMint: Keypair;

  // PDAs
  let globalConfigPda: PublicKey;
  let poolPda: PublicKey;
  let tokenAVaultPda: PublicKey;
  let tokenBVaultPda: PublicKey;

  before(async () => {
    // Airdrop SOL to test accounts
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(user.publicKey, 5 * LAMPORTS_PER_SOL)
    );
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(protocolFeeRecipient.publicKey, 2 * LAMPORTS_PER_SOL)
    );

    // Derive PDAs
    [globalConfigPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("global_config")],
      program.programId
    );

    // Create token mints
    tokenAMint = Keypair.generate();
    tokenBMint = Keypair.generate();

    const mintRent = await getMinimumBalanceForRentExemptMint(provider.connection);

    // Create and initialize token A mint
    const createTokenATx = new anchor.web3.Transaction().add(
      anchor.web3.SystemProgram.createAccount({
        fromPubkey: admin.publicKey,
        newAccountPubkey: tokenAMint.publicKey,
        space: MINT_SIZE,
        lamports: mintRent,
        programId: TOKEN_PROGRAM_ID,
      }),
      createInitializeMint2Instruction(
        tokenAMint.publicKey,
        6, // USDC decimals
        admin.publicKey,
        null,
        TOKEN_PROGRAM_ID
      )
    );

    await provider.sendAndConfirm(createTokenATx, [admin.payer, tokenAMint]);

    // Create and initialize token B mint
    const createTokenBTx = new anchor.web3.Transaction().add(
      anchor.web3.SystemProgram.createAccount({
        fromPubkey: admin.publicKey,
        newAccountPubkey: tokenBMint.publicKey,
        space: MINT_SIZE,
        lamports: mintRent,
        programId: TOKEN_PROGRAM_ID,
      }),
      createInitializeMint2Instruction(
        tokenBMint.publicKey,
        6, // USDT decimals
        admin.publicKey,
        null,
        TOKEN_PROGRAM_ID
      )
    );

    await provider.sendAndConfirm(createTokenBTx, [admin.payer, tokenBMint]);
  });

  it("Initializes the global config", async () => {
    await program.methods
      .initialize()
      .accounts({
        admin: admin.publicKey,
        globalConfig: globalConfigPda,
        protocolFeeRecipient: protocolFeeRecipient.publicKey,
        systemProgram: SystemProgram.programId,
        rent: SYSVAR_RENT_PUBKEY,
      })
      .rpc();

    const globalConfig = await program.account.globalConfig.fetch(globalConfigPda);
    expect(globalConfig.admin.toString()).to.equal(admin.publicKey.toString());
    expect(globalConfig.paused).to.be.false;

    console.log("✓ Global config initialized");
  });

  it("Initializes a pool for stablecoin pair", async () => {
    const feeTier = 5; // 0.05% for stablecoins
    const tickSpacing = 1; // Tight spacing for stables
    const initialPrice = 1_000_000; // 1.0 (scaled by 1e6)

    [poolPda] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("pool"),
        tokenAMint.publicKey.toBuffer(),
        tokenBMint.publicKey.toBuffer(),
        Buffer.from(new Uint16Array([feeTier]).buffer),
      ],
      program.programId
    );

    [tokenAVaultPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("vault_a"), poolPda.toBuffer()],
      program.programId
    );

    [tokenBVaultPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("vault_b"), poolPda.toBuffer()],
      program.programId
    );

    await program.methods
      .initializePool(feeTier, tickSpacing, new anchor.BN(initialPrice))
      .accounts({
        creator: admin.publicKey,
        globalConfig: globalConfigPda,
        tokenAMint: tokenAMint.publicKey,
        tokenBMint: tokenBMint.publicKey,
        pool: poolPda,
        tokenAVault: tokenAVaultPda,
        tokenBVault: tokenBVaultPda,
        feeTier: feeTier,
        initialPrice: new anchor.BN(initialPrice),
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        rent: SYSVAR_RENT_PUBKEY,
      })
      .rpc();

    const pool = await program.account.pool.fetch(poolPda);
    expect(pool.tokenAMint.toString()).to.equal(tokenAMint.publicKey.toString());
    expect(pool.tokenBMint.toString()).to.equal(tokenBMint.publicKey.toString());
    expect(pool.feeTier).to.equal(feeTier);
    expect(pool.tickSpacing).to.equal(tickSpacing);
    expect(pool.liquidity.toNumber()).to.equal(0);

    console.log(`✓ Pool initialized: Fee ${feeTier} bps, Tick Spacing ${tickSpacing}`);
  });

  it("Adds liquidity to a position", async () => {
    // Create user token accounts
    const userTokenAAccount = getAssociatedTokenAddressSync(
      tokenAMint.publicKey,
      user.publicKey,
      false,
      TOKEN_PROGRAM_ID
    );

    const userTokenBAccount = getAssociatedTokenAddressSync(
      tokenBMint.publicKey,
      user.publicKey,
      false,
      TOKEN_PROGRAM_ID
    );

    // Create token accounts if they don't exist
    try {
      const createATx = new anchor.web3.Transaction().add(
        createAssociatedTokenAccountInstruction(
          user.publicKey,
          userTokenAAccount,
          user.publicKey,
          tokenAMint.publicKey,
          TOKEN_PROGRAM_ID
        )
      );
      await provider.sendAndConfirm(createATx, [user]);
    } catch (err) {
      // Account might already exist
    }

    try {
      const createBTx = new anchor.web3.Transaction().add(
        createAssociatedTokenAccountInstruction(
          user.publicKey,
          userTokenBAccount,
          user.publicKey,
          tokenBMint.publicKey,
          TOKEN_PROGRAM_ID
        )
      );
      await provider.sendAndConfirm(createBTx, [user]);
    } catch (err) {
      // Account might already exist
    }

    // Mint tokens to user
    const mintAmount = 1_000_000_000; // 1000 tokens (6 decimals)
    await provider.sendAndConfirm(
      new anchor.web3.Transaction().add(
        createMintToInstruction(
          tokenAMint.publicKey,
          userTokenAAccount,
          admin.publicKey,
          mintAmount,
          [],
          TOKEN_PROGRAM_ID
        ),
        createMintToInstruction(
          tokenBMint.publicKey,
          userTokenBAccount,
          admin.publicKey,
          mintAmount,
          [],
          TOKEN_PROGRAM_ID
        )
      ),
      [admin.payer]
    );

    // Add liquidity
    const tickLower = -100;
    const tickUpper = 100;
    const amountA = 100_000_000; // 100 tokens
    const amountB = 100_000_000; // 100 tokens

    await program.methods
      .addLiquidity(tickLower, tickUpper, new anchor.BN(amountA), new anchor.BN(amountB))
      .accounts({
        owner: user.publicKey,
        globalConfig: globalConfigPda,
        pool: poolPda,
        position: PublicKey.findProgramAddressSync(
          [
            Buffer.from("position"),
            poolPda.toBuffer(),
            user.publicKey.toBuffer(),
            Buffer.from(new Int32Array([tickLower]).buffer),
            Buffer.from(new Int32Array([tickUpper]).buffer),
          ],
          program.programId
        )[0],
        ownerTokenAAccount: userTokenAAccount,
        ownerTokenBAccount: userTokenBAccount,
        tokenAVault: tokenAVaultPda,
        tokenBVault: tokenBVaultPda,
        tokenAMint: tokenAMint.publicKey,
        tokenBMint: tokenBMint.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([user])
      .rpc();

    const pool = await program.account.pool.fetch(poolPda);
    expect(pool.liquidity.toNumber()).to.be.greaterThan(0);

    console.log(`✓ Liquidity added: ${pool.liquidity.toString()}`);
  });

  it("Executes a swap", async () => {
    const userTokenAAccount = getAssociatedTokenAddressSync(
      tokenAMint.publicKey,
      user.publicKey,
      false,
      TOKEN_PROGRAM_ID
    );

    const userTokenBAccount = getAssociatedTokenAddressSync(
      tokenBMint.publicKey,
      user.publicKey,
      false,
      TOKEN_PROGRAM_ID
    );

    // Get initial balances
    const initialBalanceA = await provider.connection.getTokenAccountBalance(userTokenAAccount);
    const initialBalanceB = await provider.connection.getTokenAccountBalance(userTokenBAccount);

    // Execute swap: token A -> token B
    const amountIn = 10_000_000; // 10 tokens
    const minAmountOut = 9_000_000; // 9 tokens (slippage protection)

    await program.methods
      .swap(
        new anchor.BN(amountIn),
        new anchor.BN(minAmountOut),
        false // exact input
      )
      .accounts({
        user: user.publicKey,
        globalConfig: globalConfigPda,
        pool: poolPda,
        userTokenInAccount: userTokenAAccount,
        userTokenOutAccount: userTokenBAccount,
        tokenAVault: tokenAVaultPda,
        tokenBVault: tokenBVaultPda,
        tokenInMint: tokenAMint.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([user])
      .rpc();

    // Verify balances changed
    const finalBalanceA = await provider.connection.getTokenAccountBalance(userTokenAAccount);
    const finalBalanceB = await provider.connection.getTokenAccountBalance(userTokenBAccount);

    expect(Number(finalBalanceA.value.amount)).to.be.lessThan(Number(initialBalanceA.value.amount));
    expect(Number(finalBalanceB.value.amount)).to.be.greaterThan(Number(initialBalanceB.value.amount));

    console.log("✓ Swap executed successfully");
  });

  it("Removes liquidity from a position", async () => {
    const userTokenAAccount = getAssociatedTokenAddressSync(
      tokenAMint.publicKey,
      user.publicKey,
      false,
      TOKEN_PROGRAM_ID
    );

    const userTokenBAccount = getAssociatedTokenAddressSync(
      tokenBMint.publicKey,
      user.publicKey,
      false,
      TOKEN_PROGRAM_ID
    );

    const tickLower = -100;
    const tickUpper = 100;

    const [positionPda] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("position"),
        poolPda.toBuffer(),
        user.publicKey.toBuffer(),
        Buffer.from(new Int32Array([tickLower]).buffer),
        Buffer.from(new Int32Array([tickUpper]).buffer),
      ],
      program.programId
    );

    const position = await program.account.position.fetch(positionPda);
    const liquidityToRemove = position.liquidity.div(new anchor.BN(2)); // Remove half

    const initialBalanceA = await provider.connection.getTokenAccountBalance(userTokenAAccount);
    const initialBalanceB = await provider.connection.getTokenAccountBalance(userTokenBAccount);

    await program.methods
      .removeLiquidity(liquidityToRemove)
      .accounts({
        owner: user.publicKey,
        globalConfig: globalConfigPda,
        pool: poolPda,
        position: positionPda,
        ownerTokenAAccount: userTokenAAccount,
        ownerTokenBAccount: userTokenBAccount,
        tokenAVault: tokenAVaultPda,
        tokenBVault: tokenBVaultPda,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([user])
      .rpc();

    // Verify tokens were returned
    const finalBalanceA = await provider.connection.getTokenAccountBalance(userTokenAAccount);
    const finalBalanceB = await provider.connection.getTokenAccountBalance(userTokenBAccount);

    expect(Number(finalBalanceA.value.amount)).to.be.greaterThan(Number(initialBalanceA.value.amount));
    expect(Number(finalBalanceB.value.amount)).to.be.greaterThan(Number(initialBalanceB.value.amount));

    console.log("✓ Liquidity removed successfully");
  });

  it("Pauses and unpauses the protocol", async () => {
    // Pause
    await program.methods
      .pause(true)
      .accounts({
        admin: admin.publicKey,
        globalConfig: globalConfigPda,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    let globalConfig = await program.account.globalConfig.fetch(globalConfigPda);
    expect(globalConfig.paused).to.be.true;

    // Try to swap (should fail)
    try {
      await program.methods
        .swap(new anchor.BN(1000), new anchor.BN(900), false)
        .accounts({
          user: user.publicKey,
          globalConfig: globalConfigPda,
          pool: poolPda,
          userTokenInAccount: getAssociatedTokenAddressSync(
            tokenAMint.publicKey,
            user.publicKey,
            false,
            TOKEN_PROGRAM_ID
          ),
          userTokenOutAccount: getAssociatedTokenAddressSync(
            tokenBMint.publicKey,
            user.publicKey,
            false,
            TOKEN_PROGRAM_ID
          ),
          tokenAVault: tokenAVaultPda,
          tokenBVault: tokenBVaultPda,
          tokenInMint: tokenAMint.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .signers([user])
        .rpc();

      expect.fail("Should have failed when paused");
    } catch (err) {
      expect(err.toString()).to.include("ProgramPaused");
    }

    // Unpause
    await program.methods
      .pause(false)
      .accounts({
        admin: admin.publicKey,
        globalConfig: globalConfigPda,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    globalConfig = await program.account.globalConfig.fetch(globalConfigPda);
    expect(globalConfig.paused).to.be.false;

    console.log("✓ Pause/unpause works correctly");
  });
});
