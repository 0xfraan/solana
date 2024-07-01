import * as anchor from "@coral-xyz/anchor";
import {Game} from '../target/types/game';
import {formatValue, GAME_CONFIG_SEED} from './utils'


(async () => {
    const provider = anchor.AnchorProvider.env()
    anchor.setProvider(provider)

    const program: anchor.Program<Game> = anchor.workspace.Game;

    const payer = (provider.wallet as anchor.Wallet).payer;
    console.log(`PAYER: ${payer.publicKey}`);

    const [gameConfigPubKey] = anchor.web3.PublicKey.findProgramAddressSync(
        [Buffer.from(GAME_CONFIG_SEED)], program.programId
    );
    console.log(`CONFIG: ${gameConfigPubKey}`);

    const tx = await program.methods
        .setAmounts(new anchor.BN(1_000_000), new anchor.BN(50_000_000), new anchor.BN(255_000_000))
        .accounts({
            payer: payer.publicKey,
            gameConfig: gameConfigPubKey,
        })
        .signers([payer])
        .rpc();

    console.log(`[TX] migrate: ${tx}`);
    const config = await program.account.gameConfig.fetch(gameConfigPubKey)
    console.log("Game config:", formatValue(config));
})().then(() => console.log("Finished successfully")).catch(console.error)
