# Dockerfile para loci2d server
FROM rust:nightly-bookworm

WORKDIR /app

# Instalar dependências do sistema
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copiar arquivos do projeto
COPY . .

# Compilar o projeto
RUN cargo build --release

# Expor porta UDP 8080
EXPOSE 8080/udp

# Rodar o servidor
CMD ["./target/release/loci2d"]
