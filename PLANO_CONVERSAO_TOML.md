# Plano de Conversão JSON → TOML e Integração com `mat create`

## 1. Estrutura TOML Proposta

### 1.1 Arquivo de Configuração: `.mat/settings.toml`

```toml
# Configurações de worktree
[worktree]
# Padrões de arquivos para copiar ao criar worktree
copy_patterns = [
    ".env*",
    ".vscode/**"
]

# Padrões de arquivos para ignorar na cópia
copy_ignores = [
    "**/dist/**",
    "**/node_modules/**",
    "**/.git/**",
    "**/Thumbs.db",
    "**/.DS_Store"
]

# Template do caminho da worktree
# Variáveis disponíveis: $BASE_PATH, $APP_NAME, $TYPE, $NAME
path_template = "$BASE_PATH.wtree"

# Comandos para executar após criação da worktree
post_create_cmd = ["npm install"]

# Comando para abrir terminal (vazio = usar padrão do sistema)
terminal_command = ""

# Deletar branch junto com worktree
delete_branch_with_worktree = false

# Futuras seções podem ser adicionadas aqui:
# [branch]
# [naming]
# [hooks]
# etc.
```

### 1.2 Estrutura Rust Correspondente

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct WorktreeSettings {
    #[serde(default = "default_worktree_copy_patterns")]
    pub copy_patterns: Vec<String>,
    
    #[serde(default = "default_worktree_copy_ignores")]
    pub copy_ignores: Vec<String>,
    
    #[serde(default = "default_worktree_path_template")]
    pub path_template: String,
    
    #[serde(default)]
    pub post_create_cmd: Vec<String>,
    
    #[serde(default)]
    pub terminal_command: String,
    
    #[serde(default)]
    pub delete_branch_with_worktree: bool,
}

impl Default for WorktreeSettings {
    fn default() -> Self {
        WorktreeSettings {
            copy_patterns: default_worktree_copy_patterns(),
            copy_ignores: default_worktree_copy_ignores(),
            path_template: default_worktree_path_template(),
            post_create_cmd: vec!["npm install".to_string()],
            terminal_command: String::new(),
            delete_branch_with_worktree: false,
        }
    }
}

fn default_worktree_copy_patterns() -> Vec<String> {
    vec![".env*".to_string(), ".vscode/**".to_string()]
}

fn default_worktree_copy_ignores() -> Vec<String> {
    vec![
        "**/dist/**".to_string(),
        "**/node_modules/**".to_string(),
        "**/.git/**".to_string(),
        "**/Thumbs.db".to_string(),
        "**/.DS_Store".to_string(),
    ]
}

fn default_worktree_path_template() -> String {
    "$BASE_PATH.wtree".to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub worktree: WorktreeSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            worktree: WorktreeSettings::default(),
        }
    }
}
```

### 1.3 Formato TOML com namespace

```toml
[worktree]
copy_patterns = [".env*", ".vscode/**"]
copy_ignores = ["**/dist/**", "**/node_modules/**", "**/.git/**", "**/Thumbs.db", "**/.DS_Store"]
path_template = "$BASE_PATH.wtree"
post_create_cmd = ["npm install"]
terminal_command = ""
delete_branch_with_worktree = false
```

Dessa forma, futuras configurações podem ser adicionadas em novas seções (ex: `[branch]`, `[naming]`, etc.) sem conflitar.

## 2. Localização dos Arquivos de Configuração

### 2.1 Precedência (maior para menor)
1. **Projeto**: `<repo_root>/.mat/settings.toml`
2. **Global**: `$HOME/.mat/settings.toml`
3. **Default**: Valores embutidos no código

### 2.2 Funções de Carregamento

```rust
impl Settings {
    pub fn load() -> Result<Settings, MatError> {
        // 1. Tentar carregar do projeto
        if let Ok(project_config) = Self::load_project() {
            return Ok(project_config);
        }
        
        // 2. Tentar carregar global
        if let Ok(global_config) = Self::load_global() {
            return Ok(global_config);
        }
        
        // 3. Criar default no projeto se nenhum existe
        Self::create_default()?;
        
        // 4. Retornar default
        Ok(Settings::default())
    }
    
    fn load_project() -> Result<Settings, MatError> {
        let repo_root = get_repo_root()?;
        let path = repo_root.join(".mat").join("settings.toml");
        
        if !path.exists() {
            return Err(MatError::ConfigNotFound);
        }
        
        let content = fs::read_to_string(&path)?;
        let config: Settings = toml::from_str(&content)?;
        Ok(config)
    }
    
    fn load_global() -> Result<Settings, MatError> {
        let home = dirs::home_dir()
            .ok_or(MatError::Config { 
                key: "home_dir".into(), 
                reason: "Could not determine home directory".into() 
            })?;
        
        let path = home.join(".mat").join("settings.toml");
        
        if !path.exists() {
            return Err(MatError::ConfigNotFound);
        }
        
        let content = fs::read_to_string(&path)?;
        let config: Settings = toml::from_str(&content)?;
        Ok(config)
    }
    
    fn create_default() -> Result<(), MatError> {
        let repo_root = get_repo_root()?;
        let dir = repo_root.join(".mat");
        let path = dir.join("settings.toml");
        
        if path.exists() {
            return Ok(());
        }
        
        fs::create_dir_all(&dir)?;
        
        let default_content = r#"# Mat Settings
# See documentation for more details

[worktree]
copy_patterns = [
    ".env*",
    ".vscode/**"
]

copy_ignores = [
    "**/dist/**",
    "**/node_modules/**",
    "**/.git/**",
    "**/Thumbs.db",
    "**/.DS_Store"
]

path_template = "$BASE_PATH.wtree"

post_create_cmd = ["npm install"]

terminal_command = ""

delete_branch_with_worktree = false
"#;
        
        fs::write(&path, default_content)?;
        print_info(&format!("Created default settings at {}", path.display()));
        
        Ok(())
    }
}
```

## 3. Integração com `mat create`

### 3.1 Modificações em `src/commands/create.rs`

```rust
pub fn handle_create<R: CommandRunner>(
    task_type: &str,
    task_name: &str,
    source: Option<&str>,
    no_worktree: bool,
    use_tmux_flag: bool,
    config: &Config,
    settings: &Settings,  // NOVO PARÂMETRO
    git: &GitClient<R>,
    tmux: &TmuxClient<R>,
    app_name: &str,
    repo_dir: &Path,
) -> Result<(), MatError> {
    // ... código existente ...
    
    let names = naming::generate_names(app_name, task_type, task_name, config, repo_dir);
    
    if no_worktree {
        handle_no_worktree(git, &names, &source_branch)
    } else if should_use_tmux(config, use_tmux_flag) {
        handle_worktree_tmux(git, tmux, &names, &source_branch, settings)
    } else {
        handle_worktree_shell(git, &names, &source_branch, settings)
    }
}
```

### 3.2 Novas Funções de Utilidade

```rust
// Copiar arquivos baseado nos padrões
fn copy_worktree_files(
    source_dir: &Path,
    target_dir: &Path,
    settings: &Settings,
) -> Result<(), MatError> {
    use glob::glob;
    
    let patterns = &settings.worktree.copy_patterns;
    let ignores = &settings.worktree.copy_ignores;
    
    for pattern in patterns {
        let full_pattern = source_dir.join(pattern);
        let pattern_str = full_pattern.to_string_lossy();
        
        for entry in glob(&pattern_str).map_err(|e| MatError::Glob(e))? {
            match entry {
                Ok(path) => {
                    if should_ignore(&path, ignores) {
                        continue;
                    }
                    
                    let relative = path.strip_prefix(source_dir)?;
                    let target = target_dir.join(relative);
                    
                    if path.is_file() {
                        if let Some(parent) = target.parent() {
                            fs::create_dir_all(parent)?;
                        }
                        fs::copy(&path, &target)?;
                    }
                }
                Err(e) => eprintln!("Error: {:?}", e),
            }
        }
    }
    
    Ok(())
}

fn should_ignore(path: &Path, ignores: &[String]) -> bool {
    use glob::Pattern;
    
    let path_str = path.to_string_lossy();
    
    for ignore in ignores {
        if let Ok(pattern) = Pattern::new(ignore) {
            if pattern.matches(&path_str) {
                return true;
            }
        }
    }
    
    false
}

// Executar comandos pós-criação
fn run_post_create_commands(
    worktree_path: &Path,
    commands: &[String],
) -> Result<(), MatError> {
    for cmd in commands {
        if cmd.trim().is_empty() {
            continue;
        }
        
        print_info(&format!("Running: {}", cmd));
        
        let status = Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .current_dir(worktree_path)
            .status()?;
        
        if !status.success() {
            print_warning(&format!("Command failed with exit code: {:?}", status.code()));
        }
    }
    
    Ok(())
}

// Processar template de caminho
fn process_path_template(
    template: &str,
    base_path: &Path,
    app_name: &str,
    task_type: &str,
    task_name: &str,
) -> PathBuf {
    let path_str = template
        .replace("$BASE_PATH", &base_path.to_string_lossy())
        .replace("$APP_NAME", app_name)
        .replace("$TYPE", task_type)
        .replace("$NAME", task_name);
    
    PathBuf::from(path_str)
}
```

### 3.3 Fluxo Atualizado

```rust
fn handle_worktree_tmux<R: CommandRunner>(
    git: &GitClient<R>,
    tmux: &TmuxClient<R>,
    names: &naming::Names,
    source_branch: &str,
    settings: &Settings,
) -> Result<(), MatError> {
    // 1. Criar worktree
    let path_str = naming::normalize_path(&names.worktree_path);
    git.worktree_add(&path_str, &names.branch_name, source_branch)?;
    
    // 2. Copiar arquivos
    let repo_root = get_repo_root()?;
    copy_worktree_files(&repo_root, &names.worktree_path, settings)?;
    print_success("Copied worktree files");
    
    // 3. Executar comandos pós-criação
    if !settings.worktree.post_create_cmd.is_empty() {
        run_post_create_commands(&names.worktree_path, &settings.worktree.post_create_cmd)?;
        print_success("Executed post-create commands");
    }
    
    // 4. Abrir tmux
    let window_index = tmux.new_window(&path_str)?;
    tmux.rename_window(&names.window_name)?;
    
    // ... resto do código ...
}
```

## 4. Modificações em `src/main.rs`

```rust
CliCmd::Create {
    task_type,
    task_name,
    source,
    no_worktree,
    use_tmux,
} => {
    let r = (|| -> Result<(), MatError> {
        let config = crate::config::Config::load()?;
        let settings = crate::config::Settings::load()?;  // NOVO
        let git = crate::git::GitClient::new(crate::git::RealRunner);
        let tmux = crate::tmux::TmuxClient::new(crate::git::RealRunner);
        let app_name = naming::get_app_name();
        let current_dir = env::current_dir()?;
        commands::create::handle_create(
            &task_type,
            &task_name,
            source.as_deref(),
            no_worktree,
            use_tmux,
            &config,
            &settings,  // NOVO PARÂMETRO
            &git,
            &tmux,
            &app_name,
            &current_dir,
        )
    })();
    r
}
```

## 5. Dependências Adicionais

### 5.1 Atualizar `Cargo.toml`

```toml
[dependencies]
clap = { version = "4.5", features = ["derive"] }
console = "0.15"
dirs = "5.0"
serde = { version = "1", features = ["derive"] }
toml = "0.8"
glob = "0.3"  # NOVO: para padrões de arquivo
```

## 6. Tratamento de Erros

### 6.1 Adicionar novos tipos de erro em `src/error.rs`

```rust
#[derive(Debug)]
pub enum MatError {
    // ... erros existentes ...
    
    Glob(glob::GlobError),
    PatternError(glob::PatternError),
    ConfigNotFound,
}

impl std::fmt::Display for MatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // ... casos existentes ...
            
            MatError::Glob(e) => write!(f, "Glob error: {}", e),
            MatError::PatternError(e) => write!(f, "Pattern error: {}", e),
            MatError::ConfigNotFound => write!(f, "Configuration file not found"),
        }
    }
}

impl From<glob::GlobError> for MatError {
    fn from(err: glob::GlobError) -> Self {
        MatError::Glob(err)
    }
}

impl From<glob::PatternError> for MatError {
    fn from(err: glob::PatternError) -> Self {
        MatError::PatternError(err)
    }
}
```

## 7. Testes

### 7.1 Testes Unitários para Settings

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_settings_default() {
        let settings = Settings::default();
        assert_eq!(settings.worktree.copy_patterns.len(), 2);
        assert_eq!(settings.worktree.copy_ignores.len(), 5);
        assert_eq!(settings.worktree.path_template, "$BASE_PATH.wtree");
        assert_eq!(settings.worktree.post_create_cmd, vec!["npm install"]);
        assert_eq!(settings.worktree.terminal_command, "");
        assert_eq!(settings.worktree.delete_branch_with_worktree, false);
    }
    
    #[test]
    fn test_settings_from_toml() {
        let toml_str = r#"
[worktree]
copy_patterns = [".env*"]
copy_ignores = ["**/dist/**"]
path_template = "$BASE_PATH.worktree"
post_create_cmd = ["npm install", "npm run build"]
terminal_command = "code"
delete_branch_with_worktree = true
"#;
        
        let settings: Settings = toml::from_str(toml_str).unwrap();
        assert_eq!(settings.worktree.copy_patterns, vec![".env*"]);
        assert_eq!(settings.worktree.copy_ignores, vec!["**/dist/**"]);
        assert_eq!(settings.worktree.post_create_cmd.len(), 2);
        assert_eq!(settings.worktree.terminal_command, "code");
        assert!(settings.worktree.delete_branch_with_worktree);
    }
    
    #[test]
    fn test_process_path_template() {
        let base = PathBuf::from("/repo");
        let result = process_path_template(
            "$BASE_PATH.wtree/$APP_NAME-$TYPE-$NAME",
            &base,
            "myapp",
            "feat",
            "login",
        );
        assert_eq!(result, PathBuf::from("/repo.wtree/myapp-feat-login"));
    }
}
```

### 7.2 Testes de Integração

```rust
#[test]
fn test_create_with_settings() {
    let temp_dir = tempfile::tempdir().unwrap();
    let repo_dir = temp_dir.path().join("repo");
    fs::create_dir_all(&repo_dir).unwrap();
    
    // Criar arquivo de configuração
    let mat_dir = repo_dir.join(".mat");
    fs::create_dir_all(&mat_dir).unwrap();
    
    let config_content = r#"
[worktree]
copy_patterns = [".env*"]
copy_ignores = []
path_template = "$BASE_PATH.wtree"
post_create_cmd = []
terminal_command = ""
delete_branch_with_worktree = false
"#;
    
    fs::write(mat_dir.join("settings.toml"), config_content).unwrap();
    
    // Criar arquivo .env para copiar
    fs::write(repo_dir.join(".env"), "TEST=value").unwrap();
    
    // Testar criação
    let settings = Settings::load().unwrap();
    assert_eq!(settings.worktree.copy_patterns, vec![".env*"]);
}
```

## 8. Documentação

### 8.1 Atualizar README.md

Adicionar seção sobre configuração de worktree:

```markdown
## Configuração

O `mat` suporta configuração personalizada através do arquivo `.mat/settings.toml`.

### Localização

O arquivo pode estar em:
- **Projeto**: `<repo>/.mat/settings.toml` (prioridade alta)
- **Global**: `$HOME/.mat/settings.toml` (prioridade baixa)

Se nenhum arquivo existir, um padrão será criado automaticamente no projeto.

### Exemplo de Configuração

```toml
[worktree]
copy_patterns = [
    ".env*",
    ".vscode/**"
]

copy_ignores = [
    "**/dist/**",
    "**/node_modules/**"
]

path_template = "$BASE_PATH.wtree"

post_create_cmd = ["npm install"]

terminal_command = ""

delete_branch_with_worktree = false
```

### Campos da seção `[worktree]`

- `copy_patterns`: Lista de padrões de arquivos para copiar
- `copy_ignores`: Lista de padrões para ignorar na cópia
- `path_template`: Template do caminho da worktree
- `post_create_cmd`: Comandos para executar após criação
- `terminal_command`: Comando personalizado para abrir terminal
- `delete_branch_with_worktree`: Se deve deletar a branch ao remover worktree
```

## 9. Considerações de Compatibilidade Windows

O `mat` suporta build nativo para Windows, portanto todas as funcionalidades devem ser testadas e funcionar em ambos os sistemas.

### 9.1 Caminhos de Arquivo

```rust
use std::path::{Path, PathBuf};

// SEMPRE usar Path/PathBuf ao invés de manipulação manual de strings
fn process_path_template(
    template: &str,
    base_path: &Path,
    app_name: &str,
    task_type: &str,
    task_name: &str,
) -> PathBuf {
    // Usar PathBuf para garantir separadores corretos
    let path_str = template
        .replace("$BASE_PATH", &base_path.to_string_lossy())
        .replace("$APP_NAME", app_name)
        .replace("$TYPE", task_type)
        .replace("$NAME", task_name);
    
    PathBuf::from(path_str)
}

// Normalizar separadores para o sistema atual
fn normalize_path_separators(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}
```

### 9.2 Execução de Comandos Shell

```rust
fn run_post_create_commands(
    worktree_path: &Path,
    commands: &[String],
) -> Result<(), MatError> {
    for cmd in commands {
        if cmd.trim().is_empty() {
            continue;
        }
        
        print_info(&format!("Running: {}", cmd));
        
        // Detectar sistema operacional e usar shell apropriado
        let status = if cfg!(target_os = "windows") {
            // Windows: tentar PowerShell primeiro, depois cmd
            Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-Command", cmd])
                .current_dir(worktree_path)
                .status()
                .or_else(|_| {
                    Command::new("cmd")
                        .args(["/C", cmd])
                        .current_dir(worktree_path)
                        .status()
                })?
        } else {
            // Unix-like: usar sh
            Command::new("sh")
                .arg("-c")
                .arg(cmd)
                .current_dir(worktree_path)
                .status()?
        };
        
        if !status.success() {
            print_warning(&format!("Command failed with exit code: {:?}", status.code()));
        }
    }
    
    Ok(())
}
```

### 9.3 Glob Patterns no Windows

```rust
fn copy_worktree_files(
    source_dir: &Path,
    target_dir: &Path,
    settings: &Settings,
) -> Result<(), MatError> {
    use glob::glob;
    
    let patterns = &settings.worktree.copy_patterns;
    let ignores = &settings.worktree.copy_ignores;
    
    for pattern in patterns {
        // Construir padrão de forma cross-platform
        let full_pattern = source_dir.join(pattern);
        let pattern_str = full_pattern.to_string_lossy();
        
        // Glob crate lida com separadores automaticamente
        for entry in glob(&pattern_str).map_err(|e| MatError::Glob(e))? {
            match entry {
                Ok(path) => {
                    if should_ignore(&path, ignores) {
                        continue;
                    }
                    
                    let relative = path.strip_prefix(source_dir)?;
                    let target = target_dir.join(relative);
                    
                    if path.is_file() {
                        if let Some(parent) = target.parent() {
                            fs::create_dir_all(parent)?;
                        }
                        fs::copy(&path, &target)?;
                    }
                }
                Err(e) => eprintln!("Error: {:?}", e),
            }
        }
    }
    
    Ok(())
}

fn should_ignore(path: &Path, ignores: &[String]) -> bool {
    use glob::Pattern;
    
    // Normalizar path para comparação consistente
    let path_str = path.to_string_lossy().replace('\\', "/");
    
    for ignore in ignores {
        // Normalizar padrão também
        let ignore_normalized = ignore.replace('\\', "/");
        if let Ok(pattern) = Pattern::new(&ignore_normalized) {
            if pattern.matches(&path_str) {
                return true;
            }
        }
    }
    
    false
}
```

### 9.4 Localização do Arquivo de Configuração

```rust
impl Settings {
    fn load_global() -> Result<Settings, MatError> {
        // Usar dirs::home_dir() que funciona cross-platform
        let home = dirs::home_dir()
            .ok_or(MatError::Config { 
                key: "home_dir".into(), 
                reason: "Could not determine home directory".into() 
            })?;
        
        // Path::join() usa separadores corretos automaticamente
        let path = home.join(".mat").join("settings.toml");
        
        if !path.exists() {
            return Err(MatError::ConfigNotFound);
        }
        
        let content = fs::read_to_string(&path)?;
        let config: Settings = toml::from_str(&content)?;
        Ok(config)
    }
}
```

### 9.5 Testes Cross-Platform

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    #[cfg(target_os = "windows")]
    fn test_path_template_windows() {
        let base = PathBuf::from(r"C:\Projects\repo");
        let result = process_path_template(
            r"$BASE_PATH.wtree\$APP_NAME-$TYPE-$NAME",
            &base,
            "myapp",
            "feat",
            "login",
        );
        assert_eq!(result, PathBuf::from(r"C:\Projects\repo.wtree\myapp-feat-login"));
    }
    
    #[test]
    #[cfg(not(target_os = "windows"))]
    fn test_path_template_unix() {
        let base = PathBuf::from("/repo");
        let result = process_path_template(
            "$BASE_PATH.wtree/$APP_NAME-$TYPE-$NAME",
            &base,
            "myapp",
            "feat",
            "login",
        );
        assert_eq!(result, PathBuf::from("/repo.wtree/myapp-feat-login"));
    }
    
    #[test]
    fn test_run_post_create_commands_echo() {
        // Comando simples que funciona em ambos os sistemas
        let commands = vec!["echo test".to_string()];
        let temp_dir = tempfile::tempdir().unwrap();
        
        let result = run_post_create_commands(temp_dir.path(), &commands);
        assert!(result.is_ok());
    }
}
```

### 9.6 Documentação de Comandos no Windows

Os usuários Windows podem usar comandos específicos no `post_create_cmd`:

```toml
# Exemplo para projetos Node.js no Windows
[worktree]
post_create_cmd = ["npm install"]

# Exemplo usando PowerShell
[worktree]
post_create_cmd = ["npm install", "npm run build"]

# Exemplo usando comandos Windows nativos
[worktree]
post_create_cmd = ["cmd /c npm install"]
```

### 9.7 Checklist de Compatibilidade Windows

- [ ] Usar `Path`/`PathBuf` para todos os caminhos de arquivo
- [ ] Usar `dirs::home_dir()` para localizar diretório home
- [ ] Detectar sistema operacional ao executar comandos shell
- [ ] Normalizar separadores de caminho ao usar glob patterns
- [ ] Testar em Windows nativo (não apenas WSL)
- [ ] Documentar diferenças de comportamento no README
- [ ] Usar `cfg!(target_os = "windows")` para código específico
- [ ] Evitar hardcoded `/` ou `\` em caminhos

## 10. Checklist de Implementação

- [ ] Adicionar `glob = "0.3"` ao `Cargo.toml`
- [ ] Criar structs `Settings` e `WorktreeSettings` em `src/config.rs`
- [ ] Implementar método `Settings::load()` com precedência projeto > global > default
- [ ] Implementar função `copy_worktree_files()` (cross-platform)
- [ ] Implementar função `run_post_create_commands()` (cross-platform)
- [ ] Implementar função `process_path_template()` (cross-platform)
- [ ] Atualizar `handle_create()` para aceitar `&Settings`
- [ ] Atualizar `handle_worktree_tmux()` e `handle_worktree_shell()`
- [ ] Adicionar novos tipos de erro em `src/error.rs`
- [ ] Atualizar `src/main.rs` para carregar `Settings`
- [ ] Escrever testes unitários (incluindo testes específicos para Windows)
- [ ] Escrever testes de integração
- [ ] **Atualizar README.md** com:
  - [ ] Documentar nova estrutura de configuração `.mat/settings.toml`
  - [ ] Explicar precedência de arquivos de configuração
  - [ ] Adicionar exemplos para Windows
  - [ ] Documentar comandos `post_create_cmd` cross-platform
  - [ ] Atualizar seção de instalação Windows se necessário
- [ ] Testar manualmente com projeto real (Linux)
- [ ] Testar manualmente com projeto real (Windows)

## 10. Exemplo de Uso

```bash
# Criar worktree com configuração padrão
mat create feat login

# O mat irá:
# 1. Criar worktree em /repo.wtree/app-feat-login
# 2. Copiar .env* e .vscode/** do repositório original
# 3. Ignorar dist/, node_modules/, .git/, etc.
# 4. Executar "npm install"
# 5. Abrir nova janela/aba do terminal
```

## 11. Atualização do README.md

O README.md deve ser atualizado para documentar a nova estrutura de configuração. Abaixo estão as seções que devem ser adicionadas ou modificadas.

### 11.1 Nova Seção: Settings Configuration

Adicionar após a seção "Configuration" existente:

```markdown
### Settings Configuration

`mat` supports advanced settings via `.mat/settings.toml` for worktree customization and post-creation hooks.

#### Settings File Location

Settings are loaded with the following precedence (highest to lowest):

1. **Project**: `<repo>/.mat/settings.toml`
2. **Global**: `$HOME/.mat/settings.toml` (or `%USERPROFILE%\.mat\settings.toml` on Windows)
3. **Default**: Built-in defaults (created automatically in project if no file exists)

#### Example `.mat/settings.toml`

```toml
[worktree]
# Files to copy when creating a worktree (glob patterns)
copy_patterns = [
    ".env*",
    ".vscode/**",
    "docker-compose.yml"
]

# Files to ignore when copying (glob patterns)
copy_ignores = [
    "**/dist/**",
    "**/node_modules/**",
    "**/.git/**",
    "**/Thumbs.db",
    "**/.DS_Store"
]

# Worktree path template
# Variables: $BASE_PATH, $APP_NAME, $TYPE, $NAME
path_template = "$BASE_PATH.wtree"

# Commands to run after worktree creation
post_create_cmd = ["npm install"]

# Custom terminal command (empty = system default)
terminal_command = ""

# Delete branch when removing worktree
delete_branch_with_worktree = false
```

#### Settings Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `copy_patterns` | `string[]` | `[".env*", ".vscode/**"]` | Glob patterns for files to copy |
| `copy_ignores` | `string[]` | `["**/dist/**", ...]` | Glob patterns to ignore |
| `path_template` | `string` | `"$BASE_PATH.wtree"` | Template for worktree path |
| `post_create_cmd` | `string[]` | `["npm install"]` | Commands to run after creation |
| `terminal_command` | `string` | `""` | Custom terminal command |
| `delete_branch_with_worktree` | `bool` | `false` | Delete branch on worktree removal |

#### Path Template Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `$BASE_PATH` | Repository root path | `/home/user/project` |
| `$APP_NAME` | Application name | `myapp` |
| `$TYPE` | Task type | `feat`, `fix`, `chore` |
| `$NAME` | Task name | `login-page` |

#### Post-Create Commands

Commands are executed in the worktree directory after creation:

```toml
# Node.js project
post_create_cmd = ["npm install", "npm run build"]

# Python project
post_create_cmd = ["pip install -r requirements.txt"]

# Rust project
post_create_cmd = ["cargo build"]

# Windows-specific (PowerShell)
post_create_cmd = ["npm install", "npm run dev"]
```

**Cross-platform compatibility**: Commands are executed using `sh -c` on Unix-like systems and `powershell -Command` on Windows. Use standard commands that work on both platforms when possible.
```

### 11.2 Atualizar Seção de Instalação Windows

Adicionar nota sobre configuração:

```markdown
### Windows (native build)

```powershell
# Prerequisites:
#   1. Install Rust from https://rustup.rs
#   2. Add Windows target:
rustup target add x86_64-pc-windows-msvc

# Build
cargo build --release --target x86_64-pc-windows-msvc

# Copy mat.exe to a directory in your PATH
copy target\x86_64-pc-windows-msvc\release\mat.exe C:\tools\mat.exe
```

#### Windows Configuration

On Windows, the global settings file is located at:

```
%USERPROFILE%\.mat\settings.toml
```

Example PowerShell commands for `post_create_cmd`:

```toml
[worktree]
post_create_cmd = ["npm install", "npm run dev"]
```

Commands are executed using PowerShell by default. You can also use `cmd /c` for batch commands:

```toml
[worktree]
post_create_cmd = ["cmd /c npm install"]
```
```

### 11.3 Adicionar Seção de Exemplos por Plataforma

```markdown
## Platform-Specific Examples

### Linux/macOS

```toml
[worktree]
copy_patterns = [".env*", ".vscode/**", "docker-compose.yml"]
post_create_cmd = ["npm install", "npm run dev"]
```

### Windows

```toml
[worktree]
copy_patterns = [".env*", ".vscode/**", "docker-compose.yml"]
post_create_cmd = ["npm install", "npm run dev"]
```

**Note**: Path separators are handled automatically. Use forward slashes (`/`) in glob patterns for consistency across platforms.
```

### 11.4 Checklist de Atualização do README

- [ ] Adicionar seção "Settings Configuration" após "Configuration"
- [ ] Documentar localização do arquivo `.mat/settings.toml`
- [ ] Explicar precedência (projeto > global > default)
- [ ] Adicionar tabela de campos com tipos e defaults
- [ ] Documentar variáveis de template de caminho
- [ ] Adicionar exemplos de `post_create_cmd` para diferentes linguagens
- [ ] Adicionar nota sobre compatibilidade cross-platform
- [ ] Atualizar seção de instalação Windows com localização do config
- [ ] Adicionar seção "Platform-Specific Examples"
- [ ] Revisar exemplos existentes para garantir consistência

## 12. Considerações Futuras

A estrutura em seções do TOML permite expandir facilmente:

```toml
# Exemplo de futuras seções
[branch]
name_template = "$TYPE/$NAME"
default_source = "main"

[naming]
separator = "/"
prefix = ""

[hooks]
pre_create_cmd = []
post_close_cmd = []

[worktree.feat]
copy_patterns = [".env*", "docker-compose.yml"]
post_create_cmd = ["npm install", "npm run dev"]

[worktree.fix]
copy_patterns = [".env*"]
post_create_cmd = ["npm install"]
```

Outras possibilidades:
1. **Suporte a variáveis de ambiente nos templates**: `$ENV_VAR`
2. **Hooks personalizados**: `pre_create_cmd`, `post_close_cmd`
3. **Configuração por tipo de tarefa**: `[worktree.feat]`, `[worktree.fix]`
4. **Templates de branch por tipo**: `branch_name_template = "feat/$NAME"`
5. **Integração com gerenciadores de pacote**: detectar automaticamente npm/yarn/pnpm
