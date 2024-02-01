<script lang="ts">
  import ProgressBar from "$/components/ProgressBar.svelte";
  import { loginCracked, loginMicrosoft } from "$/ipc/auth";

  let logging_in = false;
  let login_error: string | undefined;
  let username: string;

  const isValidUsername = (username: string) => /[a-zA-Z0-9_]{3,16}/.test(username);

  function loginOffline() {
    if (!username || logging_in) return;
    if (!isValidUsername(username)) {
      login_error = "El nombre de usuario no es valido. Puedes usar unicamente letras, numeros y guiones bajos. Mínimo 3 caracteres y máximo 16.";
      return;
    }
    logging_in = true;
    login_error = undefined;

    setTimeout(() => {
      loginCracked(username)
        .catch((e) => (login_error = String(e)))
        .finally(() => (logging_in = false));
    }, Math.random() * 200);
  }

  function loginMsa() {
    if (logging_in) return;
    logging_in = true;
    login_error = undefined;

    loginMicrosoft()
      .catch((e) => (login_error = String(e)))
      .finally(() => (logging_in = false));
  }
</script>

<main>
  <form on:submit|preventDefault={loginOffline} class="container">
    <section class="img-container">
      <img src="gelcorp-title.png" alt="" />
      {#if login_error}
        <label for="username"><i>({login_error})</i></label>
      {/if}
    </section>

    <label for="username">Usuario:</label>
    <input name="username" bind:value={username} type="text" autocomplete="off" spellcheck="false" />

    <section class="btn-container">
      <button type="submit">Iniciar sesion</button>
      <button on:click|preventDefault={loginMsa}>Iniciar sesión con Microsoft</button>
    </section>

    {#if logging_in}
      <div class="progressbar-container">
        <ProgressBar progress={-1} --height="6px" />
      </div>
    {/if}
  </form>
</main>

<style>
  main {
    background-image: url("/bedrock.png");
    background-color: #0000008a;
    background-blend-mode: multiply;
    background-size: 64px;
    image-rendering: pixelated;
    box-shadow: inset 0 -10px 80px -5px black;

    display: grid;
    place-content: center;
  }

  .img-container {
    text-align: center;
    height: 100%;
    color: #8f8f8f;
  }

  .container {
    color: white;
    background-color: #0f0f0f;
    padding: 1em;

    display: flex;
    flex-direction: column;
    align-items: stretch;
    width: 42vw;
    max-width: 340px;
  }

  .container label {
    display: block;
    font-weight: 700;
    font-size: 0.85rem;
    font-family: sans-serif;
    /* margin: 2px 0; */
    margin: 0;
    padding: 0;
  }

  .container input {
    margin: 0;
    background-color: white;
    color: black;
    border: 1px solid gray;
    outline: none;
  }

  .container img {
    height: 2.8rem;
    margin: 0 2.5em;
    margin-bottom: 1em;
  }

  .container button {
    margin-top: 5px;
    background-color: #e1e1e1;
    color: black;
    box-sizing: border-box;
  }

  .btn-container {
    display: flex;
  }

  .btn-container * {
    flex: 1;
  }

  .progressbar-container {
    margin-top: 5px;
  }
</style>
