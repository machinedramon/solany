use mpl_token_metadata::instructions::{CreateMetadataAccountV3, CreateMetadataAccountV3InstructionArgs};
use mpl_token_metadata::types::DataV2;
use mpl_token_metadata::accounts::Edition;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
// Reexported types
use solana_program::{
    pubkey::Pubkey,
    system_instruction,
    system_program,
    hash::Hash
};
use std::fs::File;
use std::io;
use std::path::Path;
use std::process::Command;
use std::{thread, time::Duration};
use chrono::{TimeZone, Utc};
use dialoguer::{theme::ColorfulTheme, Select, Input};
use indicatif::ProgressBar;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Serialize, Deserialize, Clone)]
struct State {
    step: String,
    pubkey: Option<String>,
    token_name: Option<String>,
    token_symbol: Option<String>,
    token_address: Option<String>,
    account_address: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SolanaBalance {
    value: u64,
}

#[derive(Debug, Deserialize)]
struct SolanaTransactionMeta {
    #[serde(rename = "preBalances")]
    pre_balances: Vec<u64>,
    #[serde(rename = "postBalances")]
    post_balances: Vec<u64>,
    #[serde(rename = "blockTime")]
    block_time: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct SolanaTransaction {
    transaction: SolanaTransactionData,
    meta: SolanaTransactionMeta,
}

#[derive(Debug, Deserialize)]
struct SolanaTransactionData {
    signatures: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SignatureInfo {
    signature: String
}

#[derive(Debug, Deserialize)]
struct SolanaResponse<T> {
    result: T,
}

#[derive(Debug, Deserialize)]
struct SolanaSignatureResponse {
    result: Vec<SignatureInfo>,
}

fn main() {
    let mut state = load_state();

    loop {
        clear_terminal();
        let options = vec![
            "Consultar saldo e transações da carteira",
            "Criar token",
            "Sair",
        ];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Escolha uma opção")
            .items(&options)
            .default(0)
            .interact()
            .unwrap();

        state = match selection {
            0 => {
                consultar_saldo_transacoes();
                state
            }
            1 => criar_token(state),
            2 => {
                println!("Saindo... 👋");
                break;
            }
            _ => {
                println!("⚠️ Opção inválida! Por favor, escolha uma opção válida.");
                pause();
                state
            }
        };
    }
}

fn clear_terminal() {
    print!("{esc}c", esc = 27 as char);
}

fn pause() {
    println!("Pressione Enter para continuar...");
    let mut input = String::new();
    io::stdin().read_line(&mut input).expect("Falha ao ler a linha");
}

fn consultar_saldo_transacoes() {
    clear_terminal();
    println!("🔍 Digite a chave pública da carteira:");
    let mut pubkey = String::new();
    io::stdin().read_line(&mut pubkey).expect("Falha ao ler a linha");
    let pubkey = pubkey.trim();

    let bar = ProgressBar::new_spinner();
    bar.enable_steady_tick(Duration::from_millis(100));
    bar.set_message("Consultando saldo e transações...");

    let client = Client::new();
    let balance = get_balance(&client, pubkey);
    let transactions = get_transactions(&client, pubkey);

    bar.finish_and_clear();

    clear_terminal();
    println!("💰 Saldo atual da carteira {}: {:.9} SOL", pubkey, balance);
    println!("📜 Transações:");
    println!("{:<30} {:<25} {:<10}", "Data e Hora", "Transação", "Valor (SOL)");
    println!("{:-<65}", "");
    for tx in transactions {
        if let Some(datetime) = tx.meta.block_time.and_then(|ts| Utc.timestamp_opt(ts, 0).single()) {
            let amount = (tx.meta.post_balances[0] as i64 - tx.meta.pre_balances[0] as i64) as f64 / 1_000_000_000.0;
            println!("{:<30} {:<25} {:<10}", datetime, tx.transaction.signatures[0], amount);
        }
    }
    pause();
}

fn get_balance(client: &Client, pubkey: &str) -> f64 {
    let url = "https://api.mainnet-beta.solana.com";
    let request_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getBalance",
        "params": [pubkey]
    });

    let response = client
        .post(url)
        .json(&request_body)
        .send()
        .expect("Falha ao enviar requisição")
        .json::<SolanaResponse<SolanaBalance>>()
        .expect("Falha ao parsear resposta");

    response.result.value as f64 / 1_000_000_000.0
}

fn get_transactions(client: &Client, pubkey: &str) -> Vec<SolanaTransaction> {
    let url = "https://api.mainnet-beta.solana.com";
    let request_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getConfirmedSignaturesForAddress2",
        "params": [pubkey, {"limit": 10}]
    });

    let response = client
        .post(url)
        .json(&request_body)
        .send()
        .expect("Falha ao enviar requisição")
        .json::<SolanaSignatureResponse>()
        .expect("Falha ao parsear resposta");

    let signatures = response.result;
    let mut transactions = Vec::new();

    for signature_info in signatures {
        let request_body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getConfirmedTransaction",
            "params": [signature_info.signature]
        });

        let response = client
            .post(url)
            .json(&request_body)
            .send()
            .expect("Falha ao enviar requisição")
            .json::<SolanaResponse<SolanaTransaction>>()
            .expect("Falha ao parsear resposta");

        transactions.push(response.result);
    }

    transactions
}

fn criar_token(mut state: State) -> State {
    clear_terminal();
    println!("Deseja criar uma nova carteira ou usar uma existente?");
    let options = vec!["Criar nova carteira", "Usar carteira existente"];

    let selection = Select::with_theme(&ColorfulTheme::default())
        .items(&options)
        .default(0)
        .interact()
        .unwrap();

    match selection {
        0 => {
            println!("🔑 Criando carteira SOL...");
            let pubkey = create_wallet();
            println!("🗝️ Sua nova chave pública é: {}", pubkey);
            println!("💸 Por favor, deposite pelo menos 0.002 SOL na sua carteira usando a chave pública acima.");
            println!("⏳ Aguardando depósito em tempo real...");
            state.pubkey = Some(pubkey);
            state.step = "await_deposit".to_string();
        }
        1 => {
            println!("🔍 Digite a chave pública da carteira existente:");
            let mut pubkey = String::new();
            io::stdin().read_line(&mut pubkey).expect("Falha ao ler a linha");
            let pubkey = pubkey.trim().to_string();
            state.pubkey = Some(pubkey);
            state.step = "await_deposit".to_string();
        }
        _ => {}
    }

    if state.step == "await_deposit" {
        if let Some(pubkey) = &state.pubkey {
            await_deposit(pubkey);
            state.step = "create_token".to_string();
            save_state(&state);
        }
    }

    if state.step == "create_token" {
        clear_terminal();
        println!("🏷️ Digite o nome do token:");
        let token_name: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Nome do token")
            .interact_text()
            .unwrap();

        println!("🔤 Digite a abreviação do token:");
        let token_symbol: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Abreviação do token")
            .interact_text()
            .unwrap();

        let token_address = create_token();
        println!("🏷️ O endereço do seu token {} ({}) é: {}", token_name, token_symbol, token_address);

        println!("🏦 Criando conta para o token...");
        let account_address = create_token_account(&token_address);
        println!("🏦 O endereço da sua conta de token é: {}", account_address);

        println!("🛠️ Cunhando tokens...");
        mint_tokens(&token_address, &state.pubkey.as_ref().unwrap(), 1_000_000);
        println!("🪙 Foram cunhados 1.000.000 tokens para {}", account_address);

        println!("📝 Dados necessários para criação de metadados do token:");
        
        let token_uri: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("URI do token (ex: https://meusite.com/token.json)")
            .with_initial_text("https://")
            .interact_text()
            .unwrap();
    
        let payer_private_key: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Chave privada do pagador (Base58, ex: 4k3DyjU...Ns4TmG, deixe em branco se não definida)")
            .allow_empty(true)
            .interact_text()
            .unwrap();
    
        let metadata_pda: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Endereço do metadata PDA (ex: 3zxA2d...9t7N3e)")
            .with_initial_text("Clique Enter para gerar automaticamente")
            .interact_text()
            .unwrap();    

        state.token_name = Some(token_name.clone());
        state.token_symbol = Some(token_symbol.clone());
        state.token_address = Some(token_address.clone());
        state.account_address = Some(account_address.clone());
        state.step = "done".to_string();

    // Adicionar metadados ao token
    if let (Some(token_name), Some(token_symbol), Some(token_address)) = (
        &state.token_name,
        &state.token_symbol,
        &state.token_address,
    ) {
        let rpc_client = RpcClient::new_with_commitment(
            "https://api.mainnet-beta.solana.com".to_string(),
            CommitmentConfig::confirmed(),
        );
        let payer = Keypair::from_base58_string(&payer_private_key); // Substitua com sua chave privada
        let mint = Pubkey::from_str(&token_address).expect("Erro ao converter o endereço do token");
        let metadata_pda = Pubkey::from_str(&metadata_pda).expect("Erro ao converter o endereço do metadata PDA"); // Substitua com o endereço correto do PDA
        let update_authority = payer.pubkey();
        let mint_authority = &payer;

        create_metadata_account_v3(
            &rpc_client,
            &payer,
            &mint,
            &metadata_pda,
            &update_authority,
            mint_authority,
            token_name.clone(),
            token_symbol.clone(),
            token_uri.clone(), // Substitua com o URI do seu token
        ).expect("Erro ao criar a conta de metadados do token");

            println!("📦 Metadados do token criados com sucesso!");
        }
        pause();
    }

    state
}

fn create_wallet() -> String {
    let output = Command::new("solana-keygen")
        .arg("new")
        .output()
        .expect("Falha ao criar carteira");

    let output_str = String::from_utf8_lossy(&output.stdout);
    println!("{}", output_str);

    let pubkey_line = output_str.lines().find(|line| line.contains("pubkey")).unwrap();
    let pubkey = pubkey_line.split_whitespace().last().unwrap();
    pubkey.to_string()
}

fn mint_tokens(token_address: &str, recipient_address: &str, amount: u64) {
    let output = Command::new("spl-token")
        .arg("mint")
        .arg(token_address)
        .arg(amount.to_string())
        .arg(recipient_address)
        .output()
        .expect("Falha ao cunhar tokens");

    let output_str = String::from_utf8_lossy(&output.stdout);
    println!("{}", output_str);
}

fn create_token() -> String {
    let output = Command::new("spl-token")
        .arg("create-token")
        .output()
        .expect("Falha ao criar token");

    let output_str = String::from_utf8_lossy(&output.stdout);
    println!("{}", output_str);

    let token_address_line = output_str.lines().find(|line| line.contains("Creating token")).unwrap();
    let token_address = token_address_line.split_whitespace().nth(2).unwrap();
    token_address.to_string()
}

fn create_token_account(token_address: &str) -> String {
    let output = Command::new("spl-token")
        .arg("create-account")
        .arg(token_address)
        .output()
        .expect("Falha ao criar conta de token");

    let output_str = String::from_utf8_lossy(&output.stdout);
    println!("{}", output_str);

    let account_address_line = output_str.lines().find(|line| line.contains("Creating account")).unwrap();
    let account_address = account_address_line.split_whitespace().nth(2).unwrap();
    account_address.to_string()
}

fn save_state(state: &State) {
    let file = File::create("state.json").expect("Não foi possível criar o arquivo de estado");
    serde_json::to_writer(file, state).expect("Não foi possível escrever o estado no arquivo");
}

fn load_state() -> State {
    if Path::new("state.json").exists() {
        let file = File::open("state.json").expect("Não foi possível abrir o arquivo de estado");
        let state: State = serde_json::from_reader(file).expect("Não foi possível ler o estado do arquivo");
        state
    } else {
        State {
            step: "start".to_string(),
            pubkey: None,
            token_name: None,
            token_symbol: None,
            token_address: None,
            account_address: None,
        }
    }
}

fn await_deposit(pubkey: &str) {
    let min_balance: f64 = 0.002; // Define o saldo mínimo necessário em SOL
    let min_balance_lamports: u64 = (min_balance * 1_000_000_000.0) as u64; // Converte para lamports
    let bar = ProgressBar::new_spinner();
    bar.enable_steady_tick(Duration::from_millis(100));
    bar.set_message("Aguardando depósito em tempo real...");

    loop {
        let client = Client::new();
        let balance = get_balance(&client, pubkey);
        clear_terminal();
        println!("💰 Saldo atual da carteira {}: {:.9} SOL", pubkey, balance);
        if (balance * 1_000_000_000.0) as u64 >= min_balance_lamports {
            println!("✅ Depósito recebido. Continuando...");
            bar.finish_and_clear();
            break;
        }
        thread::sleep(Duration::from_secs(1));
    }
}

fn create_metadata_account_v3(
    rpc_client: &RpcClient,
    payer: &Keypair,
    mint: &Pubkey,
    metadata: &Pubkey,
    update_authority: &Pubkey,
    mint_authority: &Keypair,
    token_name: String,
    token_symbol: String,
    token_uri: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let args = CreateMetadataAccountV3InstructionArgs {
        data: DataV2 {
            name: token_name,
            symbol: token_symbol,
            uri: token_uri,
            seller_fee_basis_points: 0,
            creators: None,
            collection: None,
            uses: None,
        },
        is_mutable: true,
        collection_details: None,
    };

    let create_metadata_account_v3_ix = CreateMetadataAccountV3 {
        metadata: *metadata,
        mint: *mint,
        mint_authority: mint_authority.pubkey(),
        payer: payer.pubkey(),
        update_authority: (*update_authority, false),
        system_program: system_program::ID,
        rent: None,
    };

    let instruction = create_metadata_account_v3_ix.instruction(args);

    let instructions = vec![
        system_instruction::create_account(
            &payer.pubkey(),
            metadata,
            rpc_client.get_minimum_balance_for_rent_exemption(Edition::LEN)?,
            Edition::LEN as u64,
            &mpl_token_metadata::ID,
        ),
        instruction,
    ];
    
    let mut transaction = Transaction::new_with_payer(&instructions, Some(&payer.pubkey()));
    let recent_blockhash: Hash = rpc_client.get_latest_blockhash()?;
    transaction.sign(&[payer, mint_authority], recent_blockhash);
    rpc_client.send_and_confirm_transaction(&transaction)?;

    Ok(())
}
