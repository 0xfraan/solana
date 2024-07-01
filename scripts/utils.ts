import * as anchor from "@coral-xyz/anchor";
import {
    AttestationQueueAccount, attestationTypes,
    DEVNET_GENESIS_HASH, FunctionAccount,
    MAINNET_GENESIS_HASH,
    SwitchboardProgram
} from "@switchboard-xyz/solana.js";
import dotenv from "dotenv";
import fs from "fs";
import path from "path";
import {parseRawMrEnclave} from "@switchboard-xyz/common";

dotenv.config();

export const GAME_CONFIG_SEED = "GAME_CONFIG";
export const GAME_STATE_SEED = "GAME_STATE";
export const BET_SEED = "BET";

export const myMrEnclave: Uint8Array | undefined = process.env.MR_ENCLAVE
    ? parseRawMrEnclave(process.env.MR_ENCLAVE)
    : fs.existsSync(path.join(__dirname, "..", "measurement.txt"))
        ? parseRawMrEnclave(
            fs
                .readFileSync(path.join(__dirname, "..", "measurement.txt"), "utf-8")
                .trim()
        )
        : undefined;

export function toLEBytesFromUInt64(uint64: number): Buffer {
    const buffer = new ArrayBuffer(8); // 64 bits = 8 bytes
    const view = new DataView(buffer);
    view.setBigUint64(0, BigInt(uint64), true); // true for little-endian
    return Buffer.from(buffer);
}

export function formatValue(value: any) {
    if (value instanceof anchor.BN) {
        // Convert BN instances to a string (or number, depending on your preference)
        return value.toString();
    } else if (value instanceof anchor.web3.PublicKey) {
        // Convert PublicKey instances to their base58 string representation
        return value.toBase58();
    } else if (Array.isArray(value)) {
        // Recursively format each element in the array
        return value.map(formatValue);
    } else if (typeof value === 'object' && value !== null) {
        // Recursively format each value in the object
        const formattedObject = {};
        for (const [key, val] of Object.entries(value)) {
            formattedObject[key] = formatValue(val);
        }
        return formattedObject;
    } else {
        // Return the value directly if it's not one of the above types
        return value;
    }
}

export async function loadDefaultQueue(switchboardProgram: SwitchboardProgram) {
    const genesisHash = await switchboardProgram.provider.connection.getGenesisHash();
    const attestationQueueAddress =
        genesisHash === MAINNET_GENESIS_HASH
            ? "2ie3JZfKcvsRLsJaP5fSo43gUo1vsurnUAtAgUdUAiDG"
            : genesisHash === DEVNET_GENESIS_HASH
                ? "CkvizjVnm2zA5Wuwan34NhVT3zFc7vqUyGnA6tuEF5aE"
                : undefined;
    if (!attestationQueueAddress) {
        throw new Error(
            `The request script currently only works on mainnet-beta or devnet (if SWITCHBOARD_FUNCTION_PUBKEY is not set in your .env file))`
        );
    }

    return new AttestationQueueAccount(
        switchboardProgram,
        attestationQueueAddress
    );
}

export async function loadSwitchboardFunctionEnv(
    switchboardProgram: SwitchboardProgram
): Promise<
    [
            FunctionAccount | undefined,
            attestationTypes.FunctionAccountData | undefined
    ]
> {
    if (process.env.SWITCHBOARD_FUNCTION_PUBKEY) {
        console.log(
            `[env] SWITCHBOARD_FUNCTION_PUBKEY: ${process.env.SWITCHBOARD_FUNCTION_PUBKEY}`
        );
        const functionAccountInfo =
            await switchboardProgram.provider.connection.getAccountInfo(
                new anchor.web3.PublicKey(process.env.SWITCHBOARD_FUNCTION_PUBKEY)
            );

        if (!functionAccountInfo) {
            console.error(
                `$SWITCHBOARD_FUNCTION_PUBKEY in your .env file is incorrect, please fix. Creating a new Switchboard Function ...`
            );
        } else {
            // We can decode the AccountInfo to reduce our network calls
            return await FunctionAccount.decode(
                switchboardProgram,
                functionAccountInfo
            );
        }
    }

    return [undefined, undefined];
}
