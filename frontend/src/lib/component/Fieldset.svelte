<script lang="ts">
    export let name: string;
    export let open: boolean = true

    const dud = () => {};

    const toggle = () => {
        open = !open
    }
</script>

<fieldset>
    <legend on:click={toggle} on:keydown={dud} on:keyup={dud}>{(open ? "▼ " : "► ") + name}</legend>
    <div style:display={open ? "inline-block" : "none"}>
        <slot></slot>
    </div>
</fieldset>
