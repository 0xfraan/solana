import * as anchor from "@coral-xyz/anchor";
import {Game} from '../target/types/game';
import {
    BET_SEED,
    GAME_CONFIG_SEED,
    GAME_STATE_SEED,
    toLEBytesFromUInt64,
    formatValue,
    loadSwitchboardFunctionEnv
} from './utils'
import {getAssociatedTokenAddress} from "@solana/spl-token";
import {AttestationQueueAccount, SwitchboardProgram} from "@switchboard-xyz/solana.js";

(async () => {
    const provider = anchor.AnchorProvider.env()
    anchor.setProvider(provider)

    const program: anchor.Program<Game> = anchor.workspace.Game;

    const payer = (provider.wallet as anchor.Wallet).payer;
    console.log(`[env] PAYER: ${payer.publicKey}`);

    const [gameConfigPubKey] = anchor.web3.PublicKey.findProgramAddressSync(
        [Buffer.from(GAME_CONFIG_SEED)], program.programId
    );
    console.log(`CONFIG: ${gameConfigPubKey}`);

    const [gameStatePubKey] = anchor.web3.PublicKey.findProgramAddressSync(
        [Buffer.from(GAME_STATE_SEED)], program.programId
    );
    console.log(`STATE: ${gameStatePubKey}`);

    const state = await program.account.gameState.fetch(gameStatePubKey)
    const betID: number = state.nextBetId.toNumber();
    console.log(`New bet ID: ${betID}`);

    const [betPubKey] = anchor.web3.PublicKey.findProgramAddressSync(
        [Buffer.from(BET_SEED), toLEBytesFromUInt64(betID)], program.programId
    );
    console.log(`BET: ${betPubKey}`);

    console.log(`TOKEN: ${process.env.TOKEN}`);
    const token = new anchor.web3.PublicKey(process.env.TOKEN)

    const gameEscrowPubKey = await getAssociatedTokenAddress(
        token,
        gameConfigPubKey,
        true,
    );
    console.log(`ESCROW: ${gameEscrowPubKey}`);

    const userTokenAccountPubKey = await getAssociatedTokenAddress(
        token,
        payer.publicKey,
        true,
    );
    console.log(`USER TOKEN ACCOUNT: ${userTokenAccountPubKey}`);

    const switchboardProgram = await SwitchboardProgram.fromProvider(provider);
    let [switchboardFunction, functionState] = await loadSwitchboardFunctionEnv(switchboardProgram);
    let attestationQueue = new AttestationQueueAccount(
        switchboardProgram,
        functionState.attestationQueue
    );

    // Create a new request account with a fresh keypair
    const switchboardRequestKeypair = anchor.web3.Keypair.generate();
    const switchboardRequestEscrowPubkey = anchor.utils.token.associatedAddress({
        mint: switchboardProgram.mint.address,
        owner: switchboardRequestKeypair.publicKey,
    });
    console.log(`REQUEST ACCOUNT: ${switchboardRequestKeypair.publicKey}`);

    const pairArray = [66, 84, 67, 85, 83, 68, 88, 88]; // Equivalent to ['B', 'T', 'C', 'U', 'S', 'D', 'X', 'X']

    const tx = await program.methods
        .placeBet(new anchor.BN(1_000_000), pairArray, 120, true)
        .accounts({
            payer: payer.publicKey,
            gameConfig: gameConfigPubKey,
            gameState: gameStatePubKey,
            bet: betPubKey,
            userTokenAccount: userTokenAccountPubKey,
            gameEscrow: gameEscrowPubKey,
            switchboard: switchboardProgram.attestationProgramId,
            switchboardState: switchboardProgram.attestationProgramState.publicKey,
            switchboardAttestationQueue: attestationQueue.publicKey,
            switchboardFunction: switchboardFunction.publicKey,
            switchboardRequest: switchboardRequestKeypair.publicKey,
            switchboardRequestEscrow: switchboardRequestEscrowPubkey,
            switchboardMint: switchboardProgram.mint.address,
        })
        .signers([switchboardRequestKeypair])
        .rpc();
    console.log(`[TX] place bet: ${tx}`);

    const new_bet = await program.account.bet.fetch(betPubKey)
    console.log("New bet:", formatValue(new_bet));
})().then(() => console.log("Finished successfully")).catch(console.error)