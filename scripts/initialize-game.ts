import * as anchor from "@coral-xyz/anchor";
import {Game} from '../target/types/game';
import {formatValue, GAME_CONFIG_SEED, GAME_STATE_SEED, loadSwitchboardFunctionEnv} from './utils'
import {SwitchboardProgram} from "@switchboard-xyz/solana.js";
import {getAssociatedTokenAddress} from "@solana/spl-token";

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

    console.log(`TOKEN: ${process.env.TOKEN}`);
    const token = new anchor.web3.PublicKey(process.env.TOKEN)

    const gameEscrowPubKey = await getAssociatedTokenAddress(
        token,
        gameConfigPubKey,
        true,
    );

    const switchboardProgram = await SwitchboardProgram.fromProvider(provider);
    let [switchboardFunction,] = await loadSwitchboardFunctionEnv(switchboardProgram);
    console.log(`SWITCHBOARD FUNCTION: ${switchboardFunction.publicKey}`);

    const tx = await program.methods
        .initialize()
        .accounts({
            payer: payer.publicKey,
            gameConfig: gameConfigPubKey,
            gameState: gameStatePubKey,
            gameEscrow: gameEscrowPubKey,
            tokenMint: token,
            authority: payer.publicKey,
            switchboardFunction: switchboardFunction.publicKey,
        })
        .signers([payer])
        .rpc();

    console.log(`[TX] initialize: ${tx}`);
    const config = await program.account.gameConfig.fetch(gameConfigPubKey)
    console.log("Game config:", formatValue(config));
    const state = await program.account.gameState.fetch(gameStatePubKey)
    console.log("Game state:", formatValue(state));
})().then(() => console.log("Finished successfully")).catch(console.error)
