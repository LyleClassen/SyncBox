import { Button } from "@astryxdesign/core/Button";
import { Card } from "@astryxdesign/core/Card";
import { Heading } from "@astryxdesign/core/Heading";
import { Stack } from "@astryxdesign/core/Stack";
import { Text } from "@astryxdesign/core/Text";
import { TextInput } from "@astryxdesign/core/TextInput";
import { Theme } from "@astryxdesign/core/theme";
import { gothicTheme } from "@astryxdesign/theme-gothic/built";
import { invoke } from "@tauri-apps/api/core";
import { useState } from "react";
import "./App.css";

function App() {
  const [greetMsg, setGreetMsg] = useState("");
  const [name, setName] = useState("");

  async function greet() {
    // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
    setGreetMsg(await invoke("greet", { name }));
  }

  return (
    <Theme theme={gothicTheme} mode="dark">
      <Stack align="center" justify="center" padding={6} minHeight="100vh">
        <Card maxWidth={480} width="100%">
          <form
            onSubmit={(e) => {
              e.preventDefault();
              greet();
            }}
          >
            <Stack gap={4}>
              <Heading level={1}>SyncBox</Heading>
              <Text type="supporting">
                Enter a name and say hello to the Tauri backend.
              </Text>
              <TextInput
                label="Name"
                value={name}
                onChange={setName}
                placeholder="Enter a name..."
              />
              <Button type="submit" variant="primary" label="Greet" />
              {greetMsg && <Text>{greetMsg}</Text>}
            </Stack>
          </form>
        </Card>
      </Stack>
    </Theme>
  );
}

export default App;
