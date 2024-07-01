import {
    FunctionAccount,
    SwitchboardProgram,
} from "@switchboard-xyz/solana.js";
import * as anchor from "@coral-xyz/anchor";
import dotenv from "dotenv";
import {loadDefaultQueue, myMrEnclave} from "./utils";

dotenv.config();

(async () => {
    if (!process.env.DOCKER_IMAGE_NAME) {
        throw new Error(
            `You need to set DOCKER_IMAGE_NAME in your .env file to create a new Switchboard Function.`
        );
    }

    const provider = anchor.AnchorProvider.env();
    anchor.setProvider(provider)
    const switchboardProgram = await SwitchboardProgram.fromProvider(provider);
    const attestationQueue = await loadDefaultQueue(switchboardProgram);

    const payer = (provider.wallet as anchor.Wallet).payer;
    console.log(`[env] PAYER: ${payer.publicKey}`);

    let [switchboardFunction, tx] = await FunctionAccount.create(
        switchboardProgram,
        {
            name: "PRICE",
            container: process.env.DOCKER_IMAGE_NAME,
            containerRegistry: "dockerhub",
            version: "latest",
            attestationQueue,
            authority: payer.publicKey,
            mrEnclave: myMrEnclave,
        }
    );

    console.log(`[TX] create switchboard function: ${tx}`);
    console.log(
        `\nMake sure to add the following to your .env file:\n\tSWITCHBOARD_FUNCTION_PUBKEY=${switchboardFunction.publicKey}\n\n`
    );
})().then(() => console.log("Finished successfully")).catch(console.error)