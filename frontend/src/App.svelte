<script lang="ts">
    import Character from './lib/module/Character.svelte'
    import Characters from './lib/module/Characters.svelte'
    import Parts from './lib/module/Parts.svelte';
    import Template from './lib/module/Template.svelte'
    import Weapon from './lib/module/Weapon.svelte';
    import { state } from './lib/data';

    (async () => {
        let res = await fetch("/api/state")
        if (res.status == 200) {
            $state = await res.json()
        }
    })()
</script>

<div class="column">
    <Template></Template>
    <Character></Character>
</div>

<div class="column">

    {#each $state.characters as character (character.name)}
    <Characters character={character}></Characters>
    {/each}

</div>

<div class="column">
    <Weapon></Weapon>
    <Parts></Parts>
</div>
