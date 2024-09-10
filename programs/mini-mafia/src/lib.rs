use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::invoke;
use anchor_lang::solana_program::system_instruction;

declare_id!("2CUvm1CM7M8EwxQXTDUGTkRFTAiiqQMwsu3ZtcdPRBsi");

#[program]
pub mod mini_mafia {
    use super::*;

    // Inicializa el juego y prepara los datos iniciales
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let game = &mut ctx.accounts.game;
        game.players = Vec::new();
        game.roles = Vec::new();
        game.turn = 0;
        game.state = GameState::WaitingForPlayers;
        game.votes = vec![0; 4];  // Asume un máximo de 4 jugadores
        Ok(())
    }

    // Los jugadores se unen al juego y depositan SOL
    pub fn join_game(ctx: Context<JoinGame>, player: Pubkey) -> Result<()> {
        let game = &mut ctx.accounts.game;
        
        // Revisa que no haya más de 4 jugadores
        require!(game.players.len() < 4, CustomError::GameFull);

        // Transfiere SOL al juego
        let lamports = 1_000_000_000; // 1 SOL, ajusta si es necesario
        invoke(
            &system_instruction::transfer(
                &ctx.accounts.user.key(),
                &ctx.accounts.game.key(),
                lamports,
            ),
            &[
                ctx.accounts.user.to_account_info(),
                ctx.accounts.game.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;

        // Añade al jugador al juego
        game.players.push(player);
        Ok(())
    }

    // Asigna roles aleatorios y comienza el juego
    pub fn start_game(ctx: Context<StartGame>) -> Result<()> {
        let game = &mut ctx.accounts.game;

        // Verifica si hay 4 jugadores
        require!(game.players.len() == 4, CustomError::NotEnoughPlayers);

        // Asigna roles: 1 culpable y 3 ciudadanos
        let mut rng = rand::thread_rng();
        game.roles = vec![Role::Citizen; 4];
        let culprit_index = rng.gen_range(0..4);
        game.roles[culprit_index] = Role::Culprit;

        game.state = GameState::InProgress;
        Ok(())
    }

    // Los jugadores votan para eliminar a alguien
    pub fn vote_player(ctx: Context<VotePlayer>, vote_for: u8) -> Result<()> {
        let game = &mut ctx.accounts.game;

        // Verifica que el juego esté en progreso
        require!(game.state == GameState::InProgress, CustomError::GameNotInProgress);

        // Actualiza los votos
        require!(vote_for < game.players.len() as u8, CustomError::InvalidVote);
        game.votes[vote_for as usize] += 1;

        Ok(())
    }

    // Ejecuta el resultado de la votación y elimina al jugador con más votos
    pub fn end_round(ctx: Context<EndRound>) -> Result<()> {
        let game = &mut ctx.accounts.game;

        // Encuentra al jugador con más votos
        let mut max_votes = 0;
        let mut player_to_eliminate = 0;
        for (i, &votes) in game.votes.iter().enumerate() {
            if votes > max_votes {
                max_votes = votes;
                player_to_eliminate = i;
            }
        }

        // Verifica si el jugador eliminado es el culpable
        if game.roles[player_to_eliminate] == Role::Culprit {
            game.state = GameState::Finished;
            msg!("Culprit eliminated! Citizens win!");
        } else {
            // Elimina al ciudadano y continúa el juego
            game.players.remove(player_to_eliminate);
            game.roles.remove(player_to_eliminate);

            if game.roles.iter().all(|&r| r == Role::Culprit) {
                game.state = GameState::Finished;
                msg!("All citizens eliminated! Culprit wins!");
            }
        }

        // Resetea los votos para la siguiente ronda
        game.votes = vec![0; game.players.len()];

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = user, space = 8 + 32 + (32 * 4) + 4 + 4 + (1 * 4) + 1)]
    pub game: Account<'info, Game>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct JoinGame<'info> {
    #[account(mut)]
    pub game: Account<'info, Game>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct StartGame<'info> {
    #[account(mut)]
    pub game: Account<'info, Game>,
}

#[derive(Accounts)]
pub struct VotePlayer<'info> {
    #[account(mut)]
    pub game: Account<'info, Game>,
    pub user: Signer<'info>,
}

#[derive(Accounts)]
pub struct EndRound<'info> {
    #[account(mut)]
    pub game: Account<'info, Game>,
}

#[account]
pub struct Game {
    players: Vec<Pubkey>, // Jugadores
    roles: Vec<Role>,     // Roles de los jugadores
    votes: Vec<u8>,       // Votos de los jugadores
    turn: u8,             // Turno actual
    state: GameState,     // Estado del juego
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum GameState {
    WaitingForPlayers,
    InProgress,
    Finished,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Citizen,
    Culprit,
}

#[error_code]
pub enum CustomError {
    #[msg("The game is already full.")]
    GameFull,
    #[msg("Not enough players to start the game.")]
    NotEnoughPlayers,
    #[msg("The game is not in progress.")]
    GameNotInProgress,
    #[msg("Invalid vote.")]
    InvalidVote,
}
