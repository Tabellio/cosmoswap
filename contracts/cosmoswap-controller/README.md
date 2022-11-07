# Cosmoswap Controller Contract

This contract is used for swapping tokens directly between users.
It accepts both native tokens and cw20 tokens.

## Steps

1. User1 initiates the swap by executing the [cosmoswap-controller](../cosmoswap-controller/README.md).
2. Swap controller creates cosmoswap contract and sends the relevant data such as swap info and funds.
3. User2 executes this contract to complete the swap by sending the requested funds.
4. cosmoswap contract takes a fee and transfers the funds to users.
