import * as anchor from "@coral-xyz/anchor";
import {Game} from '../target/types/game';
import {
    BET_SEED,
    GAME_CONFIG_SEED,
    GAME_STATE_SEED,
    toLEBytesFromUInt64,
    formatValue,
} from './utils'

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

    const [betPubKey] = anchor.web3.PublicKey.findProgramAddressSync(
        [Buffer.from(BET_SEED), toLEBytesFromUInt64(betID)], program.programId
    );
    console.log(`BET: ${betPubKey}`);

    const config = await program.account.gameConfig.fetch(gameConfigPubKey)
    console.log("CONFIG DATA:", formatValue(config));
    console.log("STATE DATA:", formatValue(state));
    const bet = await program.account.bet.fetch(betPubKey)
    console.log("LAST BET DATA:", formatValue(bet));
})().then(() => console.log("Finished successfully")).catch(console.error)