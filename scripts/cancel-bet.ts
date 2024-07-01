import * as anchor from "@coral-xyz/anchor";
import {Game} from '../target/types/game';
import {
    BET_SEED,
    GAME_CONFIG_SEED,
    GAME_STATE_SEED,
    toLEBytesFromUInt64,
    formatValue,
} from './utils'
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

    const state = await program.account.gameState.fetch(gameStatePubKey)
    const betID: number = state.nextBetId.toNumber() - 1;
    console.log(`BET ID: ${betID}`);

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

    const tx = await program.methods
        .cancelBet(new anchor.BN(betID))
        .accounts({
            payer: payer.publicKey,
            gameState: gameStatePubKey,
            gameConfig: gameConfigPubKey,
            bet: betPubKey,
            userTokenAccount: userTokenAccountPubKey,
            gameEscrow: gameEscrowPubKey,
        })
        .rpc();
    console.log(`[TX] place bet: ${tx}`);

    const new_bet = await program.account.bet.fetch(betPubKey)
    console.log("BET DATA:", formatValue(new_bet));
})().then(() => console.log("Finished successfully")).catch(console.error)