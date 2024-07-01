import * as anchor from "@coral-xyz/anchor";
import {Game} from '../target/types/game';
import {
    BET_SEED,
    GAME_STATE_SEED,
    toLEBytesFromUInt64,
    loadSwitchboardFunctionEnv, GAME_CONFIG_SEED
} from './utils'
import {
    AttestationQueueAccount, FunctionRequestAccount,
    SwitchboardProgram
} from "@switchboard-xyz/solana.js";

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
    const betID: number = state.nextBetId.toNumber() - 1;
    console.log(`BET ID: ${betID}`);

    const [betPubKey] = anchor.web3.PublicKey.findProgramAddressSync(
        [Buffer.from(BET_SEED), toLEBytesFromUInt64(betID)], program.programId
    );
    console.log(`BET: ${betPubKey}`);

    const switchboardProgram = await SwitchboardProgram.fromProvider(provider);
    let [switchboardFunction, functionState] = await loadSwitchboardFunctionEnv(switchboardProgram);
    let attestationQueue = new AttestationQueueAccount(
        switchboardProgram,
        functionState.attestationQueue
    );

    const bet = await program.account.bet.fetch(betPubKey)
    const switchboardRequest = new FunctionRequestAccount(
        switchboardProgram,
        bet.switchboardRequest
    );
    const requestState = await switchboardRequest.loadData();
    const switchboardRequestEscrowPubkey = requestState.escrow;
    console.log(`REQUEST ACCOUNT: ${switchboardRequest.publicKey}`);

    const tx = await program.methods
        .requestBetExecution(new anchor.BN(betID))
        .accounts({
            payer: payer.publicKey,
            gameConfig: gameConfigPubKey,
            bet: betPubKey,
            switchboard: switchboardProgram.attestationProgramId,
            switchboardState: switchboardProgram.attestationProgramState.publicKey,
            switchboardAttestationQueue: attestationQueue.publicKey,
            switchboardFunction: switchboardFunction.publicKey,
            switchboardRequest: switchboardRequest.publicKey,
            switchboardRequestEscrow: switchboardRequestEscrowPubkey,
        })
        .rpc();
    console.log(`[TX] place bet: ${tx}`);
})().then(() => console.log("Finished successfully")).catch(console.error)